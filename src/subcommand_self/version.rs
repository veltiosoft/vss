pub fn run(_args: noargs::RawArgs) -> noargs::Result<()> {
    let version = env!("CARGO_PKG_VERSION");

    // ビルド時のコミットハッシュ（環境変数で指定される場合）
    let commit = option_env!("VSS_COMMIT").unwrap_or("unknown");

    // プラットフォーム情報を取得
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!("vss {}", version);
    println!("commit {}", commit);
    println!("platform {}({})", os, arch);

    Ok(())
}
