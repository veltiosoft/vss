// 共通フラグ
const HELP_FLAG: noargs::FlagSpec = noargs::HELP_FLAG
    .doc("ヘルプメッセージを表示します ('--help' なら詳細、'-h' なら簡易版を表示)");
const VERSION_FLAG: noargs::FlagSpec = noargs::VERSION_FLAG.doc("バージョン情報を表示します");

// サブコマンド
const BUILD_COMMAND: noargs::CmdSpec = noargs::cmd("build").doc("サイトをビルドします");
const SERVE_COMMAND: noargs::CmdSpec =
    noargs::cmd("serve").doc("ファイルの変更を検知して自動ビルドを行います");
const NEW_COMMAND: noargs::CmdSpec =
    noargs::cmd("new").doc("サイトのテンプレートプロジェクトを生成します");
const SELF_COMMAND: noargs::CmdSpec =
    noargs::cmd("self").doc("セルフアップデート関連のコマンド");

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    // 共通系のフラグ処理
    HELP_FLAG.take_help(&mut args);

    if VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // サブコマンドで分岐する
    if BUILD_COMMAND.take(&mut args).is_present() {
        vss::subcommand_build::run(args)?;
    } else if SERVE_COMMAND.take(&mut args).is_present() {
        vss::subcommand_serve::run(args)?;
    } else if NEW_COMMAND.take(&mut args).is_present() {
        todo!();
    } else if SELF_COMMAND.take(&mut args).is_present() {
        vss::subcommand_self::run(args)?;
    } else if let Some(help) = args.finish()? {
        print!("{help}");
    }

    Ok(())
}
