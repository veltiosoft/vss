use anyhow::Result;

const REPO_OWNER: &str = "veltiosoft";
const REPO_NAME: &str = "vss";

#[cfg(windows)]
const BIN_NAME: &str = "vss.exe";

#[cfg(not(windows))]
const BIN_NAME: &str = "vss";

pub fn run(_args: noargs::RawArgs) -> noargs::Result<()> {
    println!("最新バージョンを確認しています...");

    match update() {
        Ok(status) => {
            if status.updated() {
                println!("✓ vss を {} に更新しました", status.version());
                println!("\n変更を適用するには、コマンドを再実行してください。");
            } else {
                println!("✓ すでに最新バージョン ({}) です", status.version());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("更新に失敗しました: {}", e);
            std::process::exit(1);
        }
    }
}

/// 現在のプラットフォームに対応するターゲット名を取得
///
/// self_update クレートは、ターゲット名を使って以下の形式でファイルを検索する
/// - {bin_name}_{target}.tar.gz (Linux, macOS)
/// - {bin_name}_{target}.zip (Windows)
///
/// # Returns
/// プラットフォームに応じたターゲット名 (例: "linux_amd64", "darwin_arm64")
fn get_target() -> &'static str {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => "linux_amd64",
        ("linux", "aarch64") => "linux_arm64",
        ("macos", "aarch64") => "darwin_arm64",
        ("windows", "x86_64") => "windows_amd64",
        _ => panic!("Unsupported platform: {} {}", os, arch),
    }
}

fn update() -> Result<self_update::Status> {
    let current_version = env!("CARGO_PKG_VERSION");
    let target = get_target();

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .current_version(current_version)
        .show_download_progress(true)
        .no_confirm(true)
        .build()?
        .update()?;

    Ok(status)
}
