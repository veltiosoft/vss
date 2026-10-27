use std::process::{Command, Stdio};

fn update(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vss"))
        .args(["self", "update"])
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn help_and_argument_errors_do_not_start_an_update() {
    let output = update(&["--help"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("--yes"));
    let output = update(&["--yes", "--skip-verification"]);
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("最新バージョンを確認"));
}

#[test]
fn noninteractive_update_requires_explicit_yes_before_network_access() {
    let output = update(&[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--yes"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("最新バージョンを確認"));
}

#[test]
fn production_update_remains_disabled_pending_approved_trust_roots() {
    // Deliberate rollout guard. Replace this assertion with approved-key rollout
    // coverage when the administrator actually configures production keys.
    let output = update(&["--yes"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("本番署名公開鍵が未設定"));
}
