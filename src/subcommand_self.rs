pub mod update;
pub mod version;

pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    const UPDATE_COMMAND: noargs::CmdSpec = noargs::cmd("update").doc("最新バージョンに更新します");
    const VERSION_COMMAND: noargs::CmdSpec =
        noargs::cmd("version").doc("バージョン情報を表示します");

    if UPDATE_COMMAND.take(&mut args).is_present() {
        update::run(args)?;
    } else if VERSION_COMMAND.take(&mut args).is_present() {
        version::run(args)?;
    } else {
        eprintln!("使用方法: vss self [update|version]");
        eprintln!();
        eprintln!("サブコマンド:");
        eprintln!("  update   - {}", UPDATE_COMMAND.doc);
        eprintln!("  version  - {}", VERSION_COMMAND.doc);
        std::process::exit(1);
    }

    Ok(())
}
