use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Request,
    http::Uri,
    middleware::{self, Next},
    response::Response,
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

    let app = Router::new()
        .fallback_service(tower::service_fn(move |request: Request| {
            let dist = dist_dir.read().unwrap_or_else(|e| e.into_inner()).clone();
            async move {
                ServeDir::new(dist)
                    .append_index_html_on_directories(true)
                    .oneshot(request)
                    .await
            }
        }))
        .layer(middleware::from_fn(html_fallback_middleware));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app).await.context("Server failed")?;

    Ok(())
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
