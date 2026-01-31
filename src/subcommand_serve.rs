use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Request,
    http::Uri,
    middleware::{self, Next},
    response::Response,
};
use notify::event::EventKind;
use notify_debouncer_full::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tower_http::services::ServeDir;

use crate::subcommand_build::{self, ChangedFile};

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

    // 設定を取得
    let serve_config = get_serve_config(config_path)?;
    let dist_dir = serve_config.dist.clone();
    let static_dir = serve_config.r#static.clone();
    let layouts_dir = serve_config.layouts.clone();

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let dist_path = current_dir.join(&dist_dir);

    // デバウンサーを作成（300ms の遅延）
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        None,
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                // dist ディレクトリ外の変更されたファイルを収集
                let mut changed_files: Vec<ChangedFile> = Vec::new();
                let mut has_deletion = false;

                for event in &events {
                    // Access イベント（ls による atime 更新など）は無視
                    if matches!(event.kind, EventKind::Access(_)) {
                        continue;
                    }

                    // 削除イベントかどうか判定
                    let is_remove = matches!(event.kind, EventKind::Remove(_));

                    for path in &event.paths {
                        // dist ディレクトリ内は無視
                        if path.starts_with(&dist_path) {
                            continue;
                        }

                        if is_remove {
                            // 削除されたファイル
                            has_deletion = true;
                            changed_files.push(ChangedFile::Deleted(path.clone()));
                        } else if let Some(classified) = classify_changed_file(
                            path,
                            &config_path_clone,
                            &static_dir,
                            &layouts_dir,
                        ) {
                            // Config 変更の場合は即座にフルビルド
                            if matches!(classified, ChangedFile::Config) {
                                println!("[INFO] Config changed, running full rebuild...");
                                if let Err(e) = subcommand_build::run_build(&config_path_clone) {
                                    eprintln!("[ERROR] Full rebuild failed: {:#}", e);
                                } else {
                                    println!("[INFO] Full rebuild completed");
                                }
                                return;
                            }
                            changed_files.push(classified);
                        }
                    }
                }

                if changed_files.is_empty() {
                    return;
                }

                // 変更されたファイル数を表示
                let file_count = changed_files.len();
                let file_desc = if file_count == 1 {
                    format!("1 file")
                } else {
                    format!("{} files", file_count)
                };

                println!("[INFO] {} changed, rebuilding...", file_desc);

                // 増分ビルドを試行
                match subcommand_build::run_incremental_build(&config_path_clone, &changed_files) {
                    Ok(()) => {
                        println!("[INFO] Incremental rebuild completed");
                    }
                    Err(e) => {
                        // 増分ビルドが失敗した場合（Config 変更など）はフルビルドにフォールバック
                        let error_msg = format!("{:#}", e);
                        if error_msg.contains("full rebuild required") || has_deletion {
                            println!("[INFO] Falling back to full rebuild...");
                            if let Err(e) = subcommand_build::run_build(&config_path_clone) {
                                eprintln!("[ERROR] Full rebuild failed: {:#}", e);
                            } else {
                                println!("[INFO] Full rebuild completed");
                            }
                        } else {
                            eprintln!("[ERROR] Incremental rebuild failed: {:#}", e);
                        }
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
        .watch(&current_dir, RecursiveMode::Recursive)
        .context("Failed to watch directory")?;

    // 監視を継続（このスレッドをブロック）
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// 設定ファイルから dist ディレクトリのパスを取得
fn get_dist_dir(config_path: &Path) -> Result<String> {
    let serve_config = get_serve_config(config_path)?;
    Ok(serve_config.dist)
}

/// serve コマンド用の設定情報
#[derive(Debug)]
struct ServeConfig {
    dist: String,
    r#static: String,
    layouts: String,
}

/// 設定ファイルから serve コマンドに必要な情報を取得
fn get_serve_config(config_path: &Path) -> Result<ServeConfig> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Config {
        #[serde(default = "default_dist")]
        dist: String,
        #[serde(default = "default_static")]
        r#static: String,
        #[serde(default = "default_layouts")]
        layouts: String,
    }

    fn default_dist() -> String {
        "dist".to_string()
    }

    fn default_static() -> String {
        "static".to_string()
    }

    fn default_layouts() -> String {
        "layouts".to_string()
    }

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(ServeConfig {
        dist: config.dist,
        r#static: config.r#static,
        layouts: config.layouts,
    })
}

/// 変更されたファイルを分類する
fn classify_changed_file(
    path: &Path,
    config_path: &Path,
    static_dir: &str,
    layouts_dir: &str,
) -> Option<ChangedFile> {
    let path_str = path.to_string_lossy();

    // 設定ファイルの変更
    if path == config_path {
        return Some(ChangedFile::Config);
    }

    // 静的ファイルの変更
    if path.starts_with(static_dir) || path_str.starts_with(&format!("./{}", static_dir)) {
        return Some(ChangedFile::Static(path.to_path_buf()));
    }

    // テンプレートファイルの変更
    if path.starts_with(layouts_dir) || path_str.starts_with(&format!("./{}", layouts_dir)) {
        return Some(ChangedFile::Template(path.to_path_buf()));
    }

    // Markdown ファイルの変更
    if path.extension().is_some_and(|ext| ext == "md") {
        return Some(ChangedFile::Markdown(path.to_path_buf()));
    }

    None
}
