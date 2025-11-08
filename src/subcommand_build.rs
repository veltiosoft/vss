use std::{path::PathBuf, time::Instant};

/// ビルドコマンドのエントリポイント。
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let config: Option<PathBuf> = noargs::opt("config")
        .ty("PATH")
        .example("/path/to/vss.toml")
        .doc("設定ファイルパス")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    // config が指定されていない場合はデフォルトで
    // 現在のディレクトリの vss.toml を利用する。
    let _config = match config {
        Some(p) => p,
        None => PathBuf::from("vss.toml"),
    };

    // 処理時間を計測する
    let start = Instant::now();

    let duration = start.elapsed();
    println!("build finished in {} ns", duration.as_nanos());
    Ok(())
}
