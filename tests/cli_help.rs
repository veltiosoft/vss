use std::{path::Path, process::Command};

fn help(cwd: &Path, command: &[&str], flag: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_vss"))
        .current_dir(cwd)
        .args(command)
        .arg(flag)
        // 回帰で self update が実行されても GitHub へ接続させない。
        .env("HTTPS_PROXY", "http://127.0.0.1:0")
        .env("https_proxy", "http://127.0.0.1:0")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .output()
        .unwrap();

    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{command:?} {flag}: {output:?}"
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn every_command_has_short_and_long_help_with_its_full_invocation() {
    let directory = tempfile::tempdir().unwrap();
    for (command, usage) in [
        (&[][..], "vss [OPTIONS] <COMMAND>"),
        (&["build"][..], "vss build [OPTIONS]"),
        (&["serve"][..], "vss serve [OPTIONS]"),
        (&["new"][..], "vss new [OPTIONS] <DIR>"),
        (&["self"][..], "vss self [OPTIONS] <COMMAND>"),
        (&["self", "update"][..], "vss self update [OPTIONS]"),
        (&["self", "version"][..], "vss self version [OPTIONS]"),
        (&["skill"][..], "vss skill [OPTIONS] <COMMAND>"),
        (
            &["skill", "install"][..],
            "vss skill install --dir <DIR> [OPTIONS]",
        ),
    ] {
        let usage = format!("Usage: {usage}");
        for flag in ["-h", "--help"] {
            let text = help(directory.path(), command, flag);
            assert!(
                text.lines().any(|line| line == usage),
                "{command:?} {flag}: {text}"
            );
            assert!(!text.contains("最新バージョンを確認しています"));
            assert_eq!(directory.path().read_dir().unwrap().count(), 0);
        }
    }
}

#[test]
fn build_and_serve_help_describe_the_defaults_and_preview_behavior() {
    let directory = tempfile::tempdir().unwrap();
    for command in ["build", "serve"] {
        let text = help(directory.path(), &[command], "--help");
        assert!(text.contains("--config <PATH>"));
        assert!(text.contains("[default: vss.toml]"));
    }

    let text = help(directory.path(), &["serve"], "--help");
    for detail in [
        "--port <PORT>",
        "[default: 8080]",
        "http://localhost:8080",
        "ブラウザーを自動で再読み込み",
    ] {
        assert!(text.contains(detail), "{text}");
    }
}

#[test]
fn help_takes_precedence_over_version() {
    let directory = tempfile::tempdir().unwrap();
    let text = help(directory.path(), &["--version", "self", "update"], "--help");
    assert!(text.contains("Usage: vss self update"));
}
