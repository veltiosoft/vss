pub mod update;
pub mod version;

const UPDATE_COMMAND: noargs::CmdSpec = noargs::cmd("update")
    .doc("GitHub Releases の最新バージョンに更新します\n更新前に確認プロンプトは表示しません。");
const VERSION_COMMAND: noargs::CmdSpec =
    noargs::cmd("version").doc("バージョン番号、コミット、OS と CPU アーキテクチャを表示します");

pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    if UPDATE_COMMAND.take(&mut args).is_present() {
        update::run(args)?;
    } else if VERSION_COMMAND.take(&mut args).is_present() {
        version::run(args)?;
    } else {
        crate::cli::finish(args, "self")?;
    }

    Ok(())
}
