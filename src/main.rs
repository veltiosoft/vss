// 共通フラグ
const HELP_FLAG: noargs::FlagSpec =
    noargs::HELP_FLAG.doc("ヘルプを表示します（--help は詳細、-h は概要）");
const VERSION_FLAG: noargs::FlagSpec = noargs::VERSION_FLAG.doc("バージョン番号を表示します");

// サブコマンド
const BUILD_COMMAND: noargs::CmdSpec = noargs::cmd("build").doc(
    "Markdown を HTML に変換し、静的ファイルとともに出力します\n出力先（既定: dist）はビルド時に削除して再生成します。",
);
const SERVE_COMMAND: noargs::CmdSpec = noargs::cmd("serve").doc(
    "ローカル開発サーバーを起動します\nファイルの変更を検知して再ビルドし、成功するとブラウザーを自動で再読み込みします。\n既定のプレビュー URL は http://localhost:8080 です。",
);
const NEW_COMMAND: noargs::CmdSpec = noargs::cmd("new")
    .doc("サイトの雛形を生成します\nカレントディレクトリに生成するには vss new . を使います。");
const SELF_COMMAND: noargs::CmdSpec =
    noargs::cmd("self").doc("vss 本体の更新やビルド情報の表示を行います");
const SKILL_COMMAND: noargs::CmdSpec =
    noargs::cmd("skill").doc("AI エージェント向けの同梱スキルを配置します");

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = "Markdown から静的サイトを生成します\n各コマンドの詳細は vss <COMMAND> --help で確認できます。";

    // 共通系のフラグ処理
    let help = HELP_FLAG.take_help(&mut args);
    let version = VERSION_FLAG.take(&mut args);

    if version.is_present() && !help.is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // サブコマンドで分岐する
    if BUILD_COMMAND.take(&mut args).is_present() {
        vss::subcommand_build::run(args)?;
    } else if SERVE_COMMAND.take(&mut args).is_present() {
        vss::subcommand_serve::run(args)?;
    } else if NEW_COMMAND.take(&mut args).is_present() {
        vss::subcommand_new::run(args)?;
    } else if SELF_COMMAND.take(&mut args).is_present() {
        vss::subcommand_self::run(args)?;
    } else if SKILL_COMMAND.take(&mut args).is_present() {
        vss::subcommand_skill::run(args)?;
    } else if let Some(help) = args.finish()? {
        print!("{help}");
    }

    Ok(())
}
