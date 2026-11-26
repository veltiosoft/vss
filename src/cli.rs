pub(crate) const CONFIG_OPTION: noargs::OptSpec = noargs::opt("config")
    .ty("PATH")
    .default("vss.toml")
    .doc("設定ファイルのパス。指定してもサイトルートは変わりません");

/// 引数を検証し、ヘルプを表示した場合は true を返す。
pub(crate) fn finish(args: noargs::RawArgs, command: &str) -> noargs::Result<bool> {
    let Some(help) = args.finish()? else {
        return Ok(false);
    };

    // noargs は Usage のコマンド階層を「... <末尾のコマンド名>」と省略する。
    let leaf = command.rsplit(' ').next().unwrap_or(command);
    print!(
        "{}",
        help.replacen(&format!(" ... {leaf}"), &format!(" {command}"), 1)
    );
    Ok(true)
}
