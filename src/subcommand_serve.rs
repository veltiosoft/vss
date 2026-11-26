use anyhow::{Context, Result};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures_util::{Stream, StreamExt, stream};
use notify::event::{EventKind, ModifyKind};
use notify_debouncer_full::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use std::{
    collections::HashSet,
    convert::Infallible,
    future::IntoFuture,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::subcommand_build::{self, ChangedFile, Config, IncrementalBuilder, absolute_path};

const RELOAD_SCRIPT_PATH: &str = "/__vss/reload.js";

#[derive(Clone)]
struct ServeState {
    dist: PathBuf,
    revision: u128,
    shutting_down: bool,
}

/// ビルドを実行し、処理時間を返す
fn run_build_with_timing(config_path: &Path) -> Result<Duration> {
    let start = Instant::now();
    subcommand_build::run_build(config_path)?;
    Ok(start.elapsed())
}

/// serve コマンドのエントリポイント
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let config_path: PathBuf = crate::cli::CONFIG_OPTION
        .take(&mut args)
        .then(|a| a.value().parse())?;

    let port: u16 = noargs::opt("port")
        .ty("PORT")
        .default("8080")
        .doc("開発サーバーの待ち受けポート（127.0.0.1 のみ）")
        .take(&mut args)
        .then(|a| a.value().parse())?;

    if crate::cli::finish(args, "serve")? {
        return Ok(());
    }

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
    let (state_tx, state_rx) = watch::channel(ServeState {
        dist: PathBuf::from(&builder.config.dist),
        // 一時切断中にサーバーが再起動しても、初回ビルドを識別できるようにする。
        revision: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        shutting_down: false,
    });
    let build_tx = state_tx.clone();
    std::thread::spawn(move || {
        if let Err(e) = watch_files(builder, build_tx) {
            eprintln!("[ERROR] watch: {:#}", e);
        }
    });

    // HTTP サーバーを起動
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;
    println!(
        "[INFO] serving on http://localhost:{}",
        listener.local_addr()?.port()
    );

    let mut shutdown = state_rx.clone();
    let server = axum::serve(listener, serve_app(state_rx))
        .with_graceful_shutdown(async move {
            let _ = shutdown.wait_for(|state| state.shutting_down).await;
        })
        .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result.context("Server failed"),
        signal = shutdown_signal() => {
            signal.context("Failed to listen for shutdown signal")?;
            state_tx.send_modify(|state| state.shutting_down = true);
            // SSE の最後のイベントを送ってから終了する。応答できない接続は待ち続けない。
            match tokio::time::timeout(Duration::from_secs(5), server).await {
                Ok(result) => result.context("Server failed"),
                Err(_) => {
                    eprintln!("[WARN] Timed out waiting for connections to close");
                    Ok(())
                }
            }
        }
    }
}

async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}

fn serve_app(state: watch::Receiver<ServeState>) -> Router {
    Router::new()
        .route("/__vss/events", get(reload_events))
        .route(RELOAD_SCRIPT_PATH, get(reload_script))
        .fallback(serve_file)
        .with_state(state)
        .layer(middleware::from_fn(dev_security_middleware))
}

async fn reload_events(
    State(state): State<watch::Receiver<ServeState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let events = stream::unfold(Some((state, true)), |next| async move {
        let (mut state, initial) = next?;
        if !initial && state.changed().await.is_err() {
            return None;
        }
        let revision = {
            let current = state.borrow_and_update();
            if current.shutting_down {
                let event = Event::default()
                    .event("shutdown")
                    .data("server shutting down");
                return Some((Ok(event), None));
            }
            current.revision
        };
        Some((
            Ok(Event::default().data(revision.to_string())),
            Some((state, false)),
        ))
    });
    Sse::new(events).keep_alive(KeepAlive::default())
}

async fn reload_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("live_reload.js"),
    )
}

async fn serve_file(
    State(build): State<watch::Receiver<ServeState>>,
    mut request: Request,
) -> Response {
    // ファイルを開く前に番号を取得し、古い HTML に新しい番号が付くのを防ぐ。
    let build = build.borrow().clone();
    add_html_extension(&mut request);
    let host = request.headers()[header::HOST].clone();
    let head = request.method() == Method::HEAD;
    request.headers_mut().remove(header::IF_MODIFIED_SINCE);
    let full_request = request.headers().contains_key(header::RANGE).then(|| {
        let mut full = Request::new(Body::empty());
        *full.method_mut() = request.method().clone();
        *full.uri_mut() = request.uri().clone();
        *full.headers_mut() = request.headers().clone();
        full.headers_mut().remove(header::RANGE);
        full
    });

    let files = ServeDir::new(build.dist).append_index_html_on_directories(true);
    let mut response = files
        .clone()
        .oneshot(request)
        .await
        .unwrap()
        .into_response();
    let is_html = |response: &Response| {
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|value| value == "text/html")
    };
    if let Some(full_request) = full_request.filter(|_| is_html(&response)) {
        // HTML は配信時に変わるため、ディスク上のバイト範囲を返さない。
        response = files.oneshot(full_request).await.unwrap().into_response();
    }
    if response.status() != StatusCode::OK || !is_html(&response) {
        return response;
    }

    // 検証済みの Host を使い、ページに <base> があっても開発サーバーから JS を取得する。
    let script = format!(
        "\n<script defer src=\"http://{}{RELOAD_SCRIPT_PATH}\" data-vss-revision=\"{}\"></script>\n",
        host.to_str().unwrap(),
        build.revision,
    );
    let (mut parts, body) = response.into_parts();
    parts.headers.remove(header::ACCEPT_RANGES);
    parts.headers.remove(header::LAST_MODIFIED);
    if let Some(length) = parts
        .headers
        .remove(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok()?.parse::<u64>().ok())
        .and_then(|length| length.checked_add(script.len() as u64))
    {
        parts.headers.insert(header::CONTENT_LENGTH, length.into());
    }
    // 完全な HTML でも断片でも、末尾に追加した script はブラウザーが本文として処理する。
    let body = if head {
        body
    } else {
        Body::from_stream(
            body.into_data_stream()
                .chain(stream::once(async { Ok(Bytes::from(script)) })),
        )
    };
    Response::from_parts(parts, body)
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
        ("cache-control", "no-store"),
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

/// 拡張子のないページ URL を補完する。例: /about → /about.html
fn add_html_extension(request: &mut Request) {
    let path = request.uri().path();

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
}

/// ファイル監視とライブリロード通知
fn watch_files(mut builder: IncrementalBuilder, build: watch::Sender<ServeState>) -> Result<()> {
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
                let served_dir = {
                    let current = build.borrow();
                    if current.shutting_down {
                        return;
                    }
                    absolute_path(&root, &current.dist)
                };
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
                        build.send_modify(|published| {
                            published.dist = PathBuf::from(&builder.config.dist);
                            published.revision += 1;
                        });
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

    fn test_app() -> (tempfile::TempDir, Router, watch::Sender<ServeState>) {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("index.html"), "Home").unwrap();
        fs::write(root.path().join("about.html"), "About").unwrap();
        fs::write(root.path().join(".env"), "PRIVATE").unwrap();
        fs::create_dir(root.path().join(".well-known")).unwrap();
        fs::write(root.path().join(".well-known/security.txt"), "Contact").unwrap();
        fs::write(root.path().join(".well-known/.env"), "PRIVATE").unwrap();
        let (build, changes) = watch::channel(ServeState {
            dist: root.path().to_path_buf(),
            revision: 42,
            shutting_down: false,
        });
        (root, serve_app(changes), build)
    }

    #[tokio::test]
    async fn rejects_untrusted_missing_and_duplicate_hosts() {
        let (_root, app, _build) = test_app();
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
        let (root, app, _build) = test_app();
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
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            let html = std::str::from_utf8(&body).unwrap();
            assert!(html.starts_with("About\n"));
            assert!(html.contains(&format!("src=\"http://{host}{RELOAD_SCRIPT_PATH}\"")));
            assert!(html.contains("data-vss-revision=\"42\""));
        }
        assert_eq!(
            fs::read_to_string(root.path().join("about.html")).unwrap(),
            "About"
        );
    }

    #[tokio::test]
    async fn sse_sends_current_revision_then_updates_and_catches_up_after_reconnect() {
        async fn connect(app: Router) -> impl Stream<Item = Result<Bytes, axum::Error>> + Unpin {
            let response = get(app, "/__vss/events", &["localhost:8080"]).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers()[header::CONTENT_TYPE],
                "text/event-stream"
            );
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            response.into_body().into_data_stream()
        }

        async fn expect_event(
            stream: &mut (impl Stream<Item = Result<Bytes, axum::Error>> + Unpin),
            revision: u128,
        ) {
            let event = tokio::time::timeout(Duration::from_secs(1), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(event, format!("data: {revision}\n\n"));
        }

        let (_root, app, build) = test_app();
        let mut first = connect(app.clone()).await;
        let mut second = connect(app.clone()).await;
        expect_event(&mut first, 42).await;
        expect_event(&mut second, 42).await;
        build.send_modify(|build| build.revision = 43);
        expect_event(&mut first, 43).await;
        expect_event(&mut second, 43).await;
        drop(first);
        drop(second);

        build.send_modify(|build| build.revision = 44);
        build.send_modify(|build| build.revision = 45);
        expect_event(&mut connect(app).await, 45).await;
    }

    #[tokio::test]
    async fn sse_sends_shutdown_to_all_connections_and_then_ends() {
        let (_root, app, state) = test_app();
        let connect = || get(app.clone(), "/__vss/events", &["localhost:8080"]);
        let mut streams = Vec::new();
        for _ in 0..2 {
            let mut stream = connect().await.into_body().into_data_stream();
            let initial = tokio::time::timeout(Duration::from_secs(1), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(initial, "data: 42\n\n");
            streams.push(stream);
        }

        state.send_modify(|state| state.shutting_down = true);
        // 終了処理が始まった直後の接続にも、ビルド番号ではなく終了を通知する。
        streams.push(connect().await.into_body().into_data_stream());
        for mut stream in streams {
            let event = tokio::time::timeout(Duration::from_secs(1), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(event, "event: shutdown\ndata: server shutting down\n\n");
            assert!(
                tokio::time::timeout(Duration::from_secs(1), stream.next())
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[tokio::test]
    async fn serves_reload_client_only_for_allowed_hosts() {
        let (_root, app, _build) = test_app();
        for path in [RELOAD_SCRIPT_PATH, "/__vss/events"] {
            assert_eq!(
                get(app.clone(), path, &["other.test"]).await.status(),
                StatusCode::BAD_REQUEST
            );
        }
        let response = get(app, RELOAD_SCRIPT_PATH, &["localhost:8080"]).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap(),
            include_str!("live_reload.js")
        );
    }

    #[tokio::test]
    async fn html_head_and_range_requests_describe_the_enhanced_response() {
        let (_root, app, _build) = test_app();
        for method in [Method::GET, Method::HEAD] {
            let request = Request::builder()
                .method(method.clone())
                .uri("/")
                .header(header::HOST, "localhost:8080")
                .header(header::RANGE, "bytes=0-1")
                .header(header::IF_MODIFIED_SINCE, "Wed, 01 Jan 3000 00:00:00 GMT")
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            for header in [
                header::ACCEPT_RANGES,
                header::CONTENT_RANGE,
                header::LAST_MODIFIED,
            ] {
                assert!(!response.headers().contains_key(header));
            }
            let length: usize = response.headers()[header::CONTENT_LENGTH]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert!(length > "Home".len());
            if method == Method::HEAD {
                assert!(body.is_empty());
            } else {
                assert_eq!(body.len(), length);
                assert!(body.starts_with(b"Home"));
            }
        }
    }

    #[tokio::test]
    async fn encoded_tag_urls_reach_the_unchanged_portable_output_paths() {
        let (root, app, _build) = test_app();
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
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert!(body.starts_with(format!("{tag}\n<script").as_bytes()));
        }
    }

    #[tokio::test]
    async fn denies_hidden_and_traversal_paths_but_serves_public_well_known_files() {
        let (_root, app, _build) = test_app();
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
        let response = get(app.clone(), "/.well-known/security.txt", &["localhost"]).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap(),
            "Contact"
        );

        let request = Request::builder()
            .uri("/.well-known/security.txt")
            .header(header::HOST, "localhost")
            .header(header::RANGE, "bytes=0-2")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap(),
            "Con"
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
