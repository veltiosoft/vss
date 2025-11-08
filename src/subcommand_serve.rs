use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Request,
    http::Uri,
    middleware::{self, Next},
    response::Response,
};
use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::*};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tower_http::services::ServeDir;

use crate::subcommand_build;

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
    subcommand_build::run_build(config_path)?;
    println!("[INFO] Initial build completed");

    // 設定を読み込んで dist ディレクトリを取得
    let dist_dir = get_dist_dir(config_path)?;

    // ファイル監視を開始
    let config_path_clone = config_path.to_path_buf();
    let rebuild_flag = Arc::new(Mutex::new(false));
    let rebuild_flag_clone = rebuild_flag.clone();

    std::thread::spawn(move || {
        if let Err(e) = watch_files(&config_path_clone, rebuild_flag_clone) {
            eprintln!("[ERROR] watch: {:#}", e);
        }
    });

    // HTTP サーバーを起動
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("[INFO] serving on http://localhost:{}", port);

    let serve_dir = ServeDir::new(&dist_dir).append_index_html_on_directories(true);

    let app = Router::new()
        .fallback_service(serve_dir)
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
fn watch_files(config_path: &Path, _rebuild_flag: Arc<Mutex<bool>>) -> Result<()> {
    let config_path_clone = config_path.to_path_buf();

    // dist ディレクトリのパスを取得（絶対パスに変換）
    let dist_dir = get_dist_dir(config_path)?;
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let dist_path = current_dir.join(&dist_dir);

    // デバウンサーを作成（300ms の遅延）
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                // dist ディレクトリ以外のファイルが変更された場合のみ再ビルド
                let should_rebuild = events
                    .iter()
                    .any(|event| !event.path.starts_with(&dist_path));

                if should_rebuild {
                    println!("[INFO] File changed, rebuilding...");
                    if let Err(e) = subcommand_build::run_build(&config_path_clone) {
                        eprintln!("[ERROR] Rebuild failed: {:#}", e);
                    } else {
                        println!("[INFO] Rebuild completed");
                    }
                }
            }
            Err(errors) => {
                eprintln!("[ERROR] watch error: {:?}", errors);
            }
        },
    )
    .context("Failed to create file watcher")?;

    // 現在のディレクトリ配下を再帰的に監視
    debouncer
        .watcher()
        .watch(&current_dir, RecursiveMode::Recursive)
        .context("Failed to watch directory")?;

    // 監視を継続（このスレッドをブロック）
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// 設定ファイルから dist ディレクトリのパスを取得
fn get_dist_dir(config_path: &Path) -> Result<String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Config {
        #[serde(default = "default_dist")]
        dist: String,
    }

    fn default_dist() -> String {
        "dist".to_string()
    }

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(config.dist)
}
