use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use notify::event::{EventKind, ModifyKind};
use notify_debouncer_full::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use std::{
    collections::HashSet,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::subcommand_build::{self, ChangedFile, Config, IncrementalBuilder, absolute_path};

/// ビルドを実行し、処理時間を返す
fn run_build_with_timing(config_path: &Path) -> Result<Duration> {
    let start = Instant::now();
    subcommand_build::run_build(config_path)?;
    Ok(start.elapsed())
}

/// serve コマンドのエントリポイント
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let config: Option<PathBuf> = noargs::opt("config")
        .ty("PATH")
        .example("/path/to/vss.toml")
        .doc("設定ファイルパス")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;

    let port: Option<u16> = noargs::opt("port")
        .ty("PORT")
        .example("3000")
        .doc("ポート番号 (デフォルト: 8080)")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    // config が指定されていない場合はデフォルトで
    // 現在のディレクトリの vss.toml を利用する
    let config_path = match config {
        Some(p) => p,
        None => PathBuf::from("vss.toml"),
    };

    let port = port.unwrap_or(8080);

    // サーバー実行
    if let Err(e) = run_serve(&config_path, port) {
        eprintln!("Serve failed: {:#}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// 実際のサーブ処理
#[tokio::main]
async fn run_serve(config_path: &Path, port: u16) -> Result<()> {
    // 初回ビルド
    println!("[INFO] Running initial build...");
    let duration = run_build_with_timing(config_path)?;
    println!("[INFO] Initial build completed");
    println!("build finished in {} ms", duration.as_millis());

    let builder = IncrementalBuilder::new(config_path)?;
    let dist_dir = Arc::new(RwLock::new(PathBuf::from(&builder.config.dist)));
    let served_dist = Arc::clone(&dist_dir);
    std::thread::spawn(move || {
        if let Err(e) = watch_files(builder, served_dist) {
            eprintln!("[ERROR] watch: {:#}", e);
        }
    });

    // HTTP サーバーを起動
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("[INFO] serving on http://localhost:{}", port);

    let app = serve_app(dist_dir);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app).await.context("Server failed")?;

    Ok(())
}

fn serve_app(dist_dir: Arc<RwLock<PathBuf>>) -> Router {
    Router::new()
        .fallback_service(tower::service_fn(move |request: Request| {
            let dist = dist_dir.read().unwrap_or_else(|e| e.into_inner()).clone();
            async move {
                ServeDir::new(dist)
                    .append_index_html_on_directories(true)
                    .oneshot(request)
                    .await
            }
        }))
        .layer(middleware::from_fn(html_fallback_middleware))
        .layer(middleware::from_fn(dev_security_middleware))
}

fn has_local_host(request: &Request) -> bool {
    let mut hosts = request.headers().get_all("host").iter();
    let Some(host) = hosts.next().and_then(|value| value.to_str().ok()) else {
        return false;
    };
    if hosts.next().is_some() {
        return false;
    }
    let name = if let Some((name, port)) = host.split_once(':') {
        if port.is_empty()
            || !port.bytes().all(|b| b.is_ascii_digit())
            || port.parse::<u16>().is_err()
        {
            return false;
        }
        name
    } else {
        host
    };
    (name.eq_ignore_ascii_case("localhost") || name == "127.0.0.1")
        && request
            .uri()
            .authority()
            .is_none_or(|authority| authority.as_str().eq_ignore_ascii_case(host))
}

async fn dev_security_middleware(request: Request, next: Next) -> Response {
    let mut response = if !has_local_host(&request) {
        StatusCode::BAD_REQUEST.into_response()
    } else {
        // ServeDir と同じく一度デコードする。エンコードされた / や . も検査する。
        let decoded = percent_encoding::percent_decode_str(request.uri().path()).decode_utf8();
        let public_path = decoded.is_ok_and(|path| {
            !path.contains('\0')
                && path
                    .replace('\\', "/")
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .enumerate()
                    .all(|(index, part)| {
                        !part.starts_with('.') || (index == 0 && part == ".well-known")
                    })
        });
        if public_path {
            next.run(request).await
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    };
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
}

/// HTML 拡張子自動補完ミドルウェア
/// /about へのリクエストを /about.html にフォールバックする
async fn html_fallback_middleware(mut request: Request, next: Next) -> Response {
    let uri = request.uri().clone();
    let path = uri.path();

    // すでに .html で終わっている、または / で終わっている場合はスキップ
    // 拡張子がない場合は .html を付けたURIを試す
    if !path.ends_with(".html") && !path.ends_with('/') && !path.contains('.') {
        // .html を付けたパスを構築
        let html_path = format!("{}.html", path);

        // 新しい URI を構築
        if let Ok(new_uri) = Uri::builder().path_and_query(html_path.as_str()).build() {
            *request.uri_mut() = new_uri;
        }
    }

    next.run(request).await
}

/// ファイル監視とホットリロード
fn watch_files(mut builder: IncrementalBuilder, served_dist: Arc<RwLock<PathBuf>>) -> Result<()> {
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let watch_root = root.clone();
    let mut output_dir = absolute_path(&root, Path::new(&builder.config.dist));
    let config_path = absolute_path(&root, builder.config_path());
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        None,
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                let mut changed_files = HashSet::new();
                let served_dir = absolute_path(
                    &root,
                    &served_dist.read().unwrap_or_else(|e| e.into_inner()),
                );
                let mut has_input_event = false;
                for event in &events {
                    if event.need_rescan() {
                        changed_files.insert(ChangedFile::Config);
                    }
                    if matches!(event.kind, EventKind::Access(_)) {
                        continue;
                    }
                    for path in &event.paths {
                        let path = absolute_path(&root, path);
                        if path == root
                            || path.starts_with(&output_dir)
                            || path.starts_with(&served_dir)
                            || !path.starts_with(&root)
                        {
                            continue;
                        }
                        has_input_event = true;
                        // macOS では rename の旧パスが同時に届くとは限らない。
                        if matches!(event.kind, EventKind::Modify(ModifyKind::Name(_))) {
                            changed_files.insert(ChangedFile::Config);
                        }
                        if let Some(changed) =
                            classify_changed_file(&path, &root, &config_path, &builder.config)
                        {
                            changed_files.insert(changed);
                        }
                    }
                }
                // 通知に旧パスや rename の種類が欠けていても、消えた入力を見落とさない。
                if has_input_event && builder.has_missing_inputs() {
                    changed_files.insert(ChangedFile::Config);
                }
                if changed_files.is_empty() {
                    return;
                }
                println!(
                    "[INFO] {} files changed, rebuilding...",
                    changed_files.len()
                );
                let start = Instant::now();
                let changed_files: Vec<_> = changed_files.into_iter().collect();
                // 失敗した全体ビルドが生成したディレクトリも監視から除外する。
                if let Ok(config) = subcommand_build::load_config(builder.config_path()) {
                    output_dir = absolute_path(&root, Path::new(&config.dist));
                }
                match builder.rebuild(&changed_files) {
                    Ok(()) => {
                        output_dir = absolute_path(&root, Path::new(&builder.config.dist));
                        *served_dist.write().unwrap_or_else(|e| e.into_inner()) =
                            PathBuf::from(&builder.config.dist);
                        println!("[INFO] Rebuild completed");
                        println!("build finished in {} ms", start.elapsed().as_millis());
                    }
                    Err(e) => eprintln!("[ERROR] Rebuild failed: {:#}", e),
                }
            }
            Err(errors) => eprintln!("[ERROR] watch error: {:?}", errors),
        },
    )
    .context("Failed to create file watcher")?;
    debouncer
        .watch(&watch_root, RecursiveMode::Recursive)
        .context("Failed to watch directory")?;
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn classify_changed_file(
    path: &Path,
    root: &Path,
    config_path: &Path,
    config: &Config,
) -> Option<ChangedFile> {
    if path == config_path {
        return Some(ChangedFile::Config);
    }
    if !path.starts_with(root) {
        return None;
    }
    if !path.exists() {
        return Some(ChangedFile::Deleted(path.to_path_buf()));
    }
    if path.starts_with(absolute_path(root, Path::new(&config.r#static))) {
        return Some(ChangedFile::Static(path.to_path_buf()));
    }
    if path.starts_with(absolute_path(root, Path::new(&config.layouts))) {
        return Some(ChangedFile::Template(path.to_path_buf()));
    }
    if path.is_dir() {
        return Some(ChangedFile::Config);
    }
    if path.extension().is_some_and(|extension| extension == "md") {
        return Some(ChangedFile::Markdown(path.to_path_buf()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    async fn get(app: Router, path: &str, hosts: &[&str]) -> Response {
        let mut request = Request::builder().uri(path);
        for host in hosts {
            request = request.header("host", *host);
        }
        app.oneshot(request.body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn test_app() -> (tempfile::TempDir, Router) {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("index.html"), "Home").unwrap();
        fs::write(root.path().join("about.html"), "About").unwrap();
        fs::write(root.path().join(".env"), "PRIVATE").unwrap();
        fs::create_dir(root.path().join(".well-known")).unwrap();
        fs::write(root.path().join(".well-known/security.txt"), "Contact").unwrap();
        fs::write(root.path().join(".well-known/.env"), "PRIVATE").unwrap();
        let app = serve_app(Arc::new(RwLock::new(root.path().to_path_buf())));
        (root, app)
    }

    #[tokio::test]
    async fn rejects_untrusted_missing_and_duplicate_hosts() {
        let (_root, app) = test_app();
        for hosts in [
            vec!["evil.example"],
            vec!["localhost.evil.example:8080"],
            vec!["127.0.0.1.evil.example"],
            vec![],
            vec!["localhost", "evil.example"],
            vec!["localhost:invalid"],
            vec!["user@localhost"],
            vec!["localhost:65536"],
        ] {
            let response = get(app.clone(), "/", &hosts).await;
            assert_eq!(
                response.status(),
                axum::http::StatusCode::BAD_REQUEST,
                "hosts: {hosts:?}"
            );
        }
    }

    #[tokio::test]
    async fn serves_local_hosts_with_security_headers_and_html_fallback() {
        let (_root, app) = test_app();
        for host in [
            "localhost",
            "localhost:8080",
            "127.0.0.1",
            "127.0.0.1:3000",
            "LOCALHOST:8080",
        ] {
            let response = get(app.clone(), "/about", &[host]).await;
            assert_eq!(response.status(), axum::http::StatusCode::OK);
            assert_eq!(response.headers()["x-content-type-options"], "nosniff");
            assert_eq!(response.headers()["referrer-policy"], "no-referrer");
            assert_eq!(response.headers()["x-frame-options"], "DENY");
            assert_eq!(
                axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap(),
                "About"
            );
        }
    }

    #[tokio::test]
    async fn encoded_tag_urls_reach_the_unchanged_portable_output_paths() {
        let (root, app) = test_app();
        for (tag, url) in [
            ("news", "/tags/news/"),
            ("C++", "/tags/C%2B%2B/"),
            ("a b", "/tags/a%20b/"),
            ("日本語", "/tags/%E6%97%A5%E6%9C%AC%E8%AA%9E/"),
        ] {
            let directory = root.path().join("tags").join(tag);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("index.html"), tag).unwrap();
            let response = get(app.clone(), url, &["localhost:8080"]).await;
            assert_eq!(response.status(), StatusCode::OK, "{url}");
            assert_eq!(
                axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap(),
                tag
            );
        }
    }

    #[tokio::test]
    async fn denies_hidden_and_traversal_paths_but_serves_public_well_known_files() {
        let (_root, app) = test_app();
        for path in [
            "/.env",
            "/%2eenv",
            "/.git/config",
            "/.well-known/.env",
            "/nested%2f.env",
            "/../outside.txt",
            "/..%2foutside.txt",
            "/%2e%2e/outside.txt",
            "/%00",
        ] {
            let response = get(app.clone(), path, &["localhost:8080"]).await;
            assert_eq!(
                response.status(),
                axum::http::StatusCode::NOT_FOUND,
                "path: {path}"
            );
        }
        assert_eq!(
            get(app, "/.well-known/security.txt", &["localhost"])
                .await
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[test]
    fn classifies_absolute_paths_using_normalized_configuration() {
        let site = tempfile::tempdir().unwrap();
        let root = site.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();
        fs::write(
            root.join("custom.toml"),
            "static = './assets'\nlayouts = './templates'",
        )
        .unwrap();
        let config_path = absolute_path(&root, Path::new("./custom.toml"));
        let config = subcommand_build::load_config(&config_path).unwrap();
        for path in [
            "assets/a.css",
            "assets/readme.md",
            "templates/default.html",
            "page.md",
            "ignored.txt",
        ] {
            fs::write(root.join(path), "test").unwrap();
        }
        let classify =
            |path: &str| classify_changed_file(&root.join(path), &root, &config_path, &config);
        assert_eq!(classify("custom.toml"), Some(ChangedFile::Config));
        assert_eq!(
            classify("assets/a.css"),
            Some(ChangedFile::Static(root.join("assets/a.css")))
        );
        assert_eq!(
            classify("assets/readme.md"),
            Some(ChangedFile::Static(root.join("assets/readme.md")))
        );
        assert_eq!(
            classify("templates/default.html"),
            Some(ChangedFile::Template(root.join("templates/default.html")))
        );
        assert_eq!(
            classify("page.md"),
            Some(ChangedFile::Markdown(root.join("page.md")))
        );
        assert_eq!(
            classify("old.md"),
            Some(ChangedFile::Deleted(root.join("old.md")))
        );
        assert_eq!(
            classify("removed.directory"),
            Some(ChangedFile::Deleted(root.join("removed.directory")))
        );
        assert_eq!(classify("ignored.txt"), None);
        assert_eq!(
            classify_changed_file(
                &root.parent().unwrap().join("outside.md"),
                &root,
                &config_path,
                &config
            ),
            None
        );
    }
}
