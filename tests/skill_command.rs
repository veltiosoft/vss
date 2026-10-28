use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vss"))
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output, message: &str) {
    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(101));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_skill(parent: &Path, output: &Output) {
    assert_success(output);
    let path = parent.join("vss/SKILL.md");
    let contents = fs::read_to_string(&path).unwrap();
    assert_eq!(
        contents,
        include_str!("../skills/vss/SKILL.md").replace("\r\n", "\n")
    );
    assert!(contents.starts_with("---\nname: vss\n"));
    assert!(contents.contains("description:"));
    for topic in ["vss new", "vss build", "vss serve", "vss.toml", "base_url"] {
        assert!(contents.contains(topic), "missing {topic}");
    }
    assert!(!contents.contains('\r'));
    assert!(contents.ends_with('\n'));
    assert_eq!(fs::read_dir(parent.join("vss")).unwrap().count(), 1);
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&path.canonicalize().unwrap().display().to_string())
    );
}

#[test]
fn installs_in_a_new_relative_directory_outside_a_site() {
    for destination in ["nested/skills", "nested/agent skills"] {
        let directory = tempfile::tempdir().unwrap();
        let output = run(
            directory.path(),
            &["skill", "install", "--dir", destination],
        );
        assert_skill(&directory.path().join(destination), &output);
        assert!(!directory.path().join("vss.toml").exists());
    }
}

#[test]
fn installs_in_an_absolute_directory_with_spaces_and_unicode() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("agent skills 日本語");
    let output = run(
        directory.path(),
        &["skill", "install", "--dir", parent.to_str().unwrap()],
    );
    assert_skill(&parent, &output);
}

#[test]
fn preserves_unrelated_files_in_an_existing_parent_and_does_not_read_config() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("vss.toml"), "invalid TOML [").unwrap();
    fs::write(directory.path().join("other-skill.md"), "keep\r\n").unwrap();
    let output = run(directory.path(), &["skill", "install", "--dir", "."]);
    assert_skill(directory.path(), &output);
    assert_eq!(
        fs::read_to_string(directory.path().join("other-skill.md")).unwrap(),
        "keep\r\n"
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("vss.toml")).unwrap(),
        "invalid TOML ["
    );
}

#[test]
fn bundled_configuration_and_content_examples_build_successfully() {
    let directory = tempfile::tempdir().unwrap();
    assert_success(&run(directory.path(), &["new", "site"]));
    let site = directory.path().join("site");
    let skill = include_str!("../skills/vss/SKILL.md").replace("\r\n", "\n");
    let example = |language: &str| {
        skill
            .split_once(&format!("```{language}\n"))
            .unwrap()
            .1
            .split_once("\n```")
            .unwrap()
            .0
    };
    fs::write(site.join("vss.toml"), example("toml")).unwrap();
    fs::write(site.join("index.md"), example("markdown")).unwrap();
    fs::write(site.join("layouts/default.html"), example("html")).unwrap();
    assert_success(&run(&site, &["build"]));
    let html = fs::read_to_string(site.join("dist/index.html")).unwrap();
    assert!(html.contains("<h1>記事の本文</h1>"));
    assert!(html.contains("https://example.com/images/cover.png"));
    assert!(html.contains("href=\"/tags/news/\""));
    assert!(site.join("dist/tags/news/index.html").is_file());
}

#[test]
fn refuses_reinstallation_without_modifying_the_existing_skill() {
    let directory = tempfile::tempdir().unwrap();
    assert_success(&run(directory.path(), &["skill", "install", "--dir", "."]));
    let skill = directory.path().join("vss/SKILL.md");
    fs::write(&skill, "customized skill\r\n").unwrap();
    let output = run(directory.path(), &["skill", "install", "--dir", "."]);
    assert_failure(&output, "already exists");
    assert_eq!(fs::read_to_string(skill).unwrap(), "customized skill\r\n");
}

#[test]
fn refuses_an_existing_skill_directory_even_if_empty() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("vss")).unwrap();
    let output = run(directory.path(), &["skill", "install", "--dir", "."]);
    assert_failure(&output, "already exists");
    assert_eq!(
        fs::read_dir(directory.path().join("vss")).unwrap().count(),
        0
    );
}

#[test]
fn refuses_a_file_in_place_of_the_skill_directory() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("vss"), "keep").unwrap();
    let output = run(directory.path(), &["skill", "install", "--dir", "."]);
    assert_failure(&output, "already exists");
    assert_eq!(
        fs::read_to_string(directory.path().join("vss")).unwrap(),
        "keep"
    );
}

#[test]
fn refuses_files_in_the_parent_path() {
    for destination in ["file", "file/nested"] {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("file"), "keep").unwrap();
        let output = run(
            directory.path(),
            &["skill", "install", "--dir", destination],
        );
        assert_failure(&output, "parent directory");
        assert_eq!(
            fs::read_to_string(directory.path().join("file")).unwrap(),
            "keep"
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
fn rejects_missing_or_invalid_arguments_without_creating_files() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["skill"],
        vec!["skill", "unknown"],
        vec!["skill", "install"],
        vec!["skill", "install", "--dir"],
        vec!["skill", "install", "--dir", ""],
        vec!["skill", "install", "--dir", "skills", "extra"],
        vec!["skill", "install", "--dir", "skills", "--force"],
        vec!["skill", "install", "--dir", "skills", "--unknown"],
        vec!["skill", "install", "--dir", "skills", "--dir", "other"],
    ] {
        let output = run(directory.path(), &args);
        assert_failure(&output, "");
        assert_eq!(
            fs::read_dir(directory.path()).unwrap().count(),
            0,
            "{args:?}"
        );
    }
    assert_failure(&run(directory.path(), &["skill", "install"]), "--dir");
}

#[test]
fn help_describes_the_commands_without_creating_files() {
    let directory = tempfile::tempdir().unwrap();
    for (args, expected) in [
        (vec!["--help"], "skill"),
        (vec!["skill", "--help"], "install"),
        (vec!["skill", "-h"], "install"),
        (vec!["skill", "install", "--help"], "--dir"),
        (vec!["skill", "install", "-h"], "--dir"),
        (
            vec!["skill", "install", "--dir", "skills", "--help"],
            "vss/SKILL.md",
        ),
    ] {
        let output = run(directory.path(), &args);
        assert_success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn refuses_existing_symlinks_including_broken_links_without_touching_targets() {
    use std::os::unix::fs::symlink;
    for broken in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = if broken {
            outside.path().join("missing")
        } else {
            outside.path().to_path_buf()
        };
        symlink(&target, directory.path().join("vss")).unwrap();
        let output = run(directory.path(), &["skill", "install", "--dir", "."]);
        assert_failure(&output, "already exists");
        assert!(
            fs::symlink_metadata(directory.path().join("vss"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn reports_permission_errors_without_creating_a_skill() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o555)).unwrap();
    // root など書き込み制限を回避できる環境では、このテストを実行しない。
    if fs::write(directory.path().join("probe"), "probe").is_ok() {
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let output = run(directory.path(), &["skill", "install", "--dir", "."]);
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert_failure(&output, "Permission denied");
    assert!(!directory.path().join("vss").exists());
}
