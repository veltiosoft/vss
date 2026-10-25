use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};
use walkdir::WalkDir;

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

fn tree(directory: &Path) -> BTreeMap<String, Vec<u8>> {
    WalkDir::new(directory)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let name = entry
                .path()
                .strip_prefix(directory)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (name, fs::read(entry.path()).unwrap())
        })
        .collect()
}

fn assert_buildable_site(directory: &Path) {
    let files = tree(directory);
    assert_eq!(
        files.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            ".gitignore",
            "README.md",
            "index.md",
            "layouts/default.html",
            "layouts/tags/default.html",
            "static/style.css",
            "vss.toml"
        ]
    );
    for (name, contents) in &files {
        assert!(!contents.contains(&b'\r'), "CR byte in {name}");
        assert!(contents.ends_with(b"\n"), "missing final LF in {name}");
        std::str::from_utf8(contents).unwrap();
    }
    assert_success(&run(directory, &["build"]));
    let index = fs::read_to_string(directory.join("dist/index.html")).unwrap();
    assert!(index.contains("<h1>Welcome to vss</h1>"));
    assert!(index.contains("<title>Home | My vss site</title>"));
    assert!(!index.contains("property=\"og:image\""));
    assert!(directory.join("dist/style.css").is_file());
    assert!(!directory.join("dist/README.html").exists());
}

#[test]
fn generates_and_builds_a_site_in_a_new_nested_directory() {
    let directory = tempfile::tempdir().unwrap();
    assert_success(&run(directory.path(), &["new", "nested/fresh-site"]));
    assert_buildable_site(&directory.path().join("nested/fresh-site"));
}

#[test]
fn generates_and_builds_a_site_in_an_empty_existing_directory() {
    let directory = tempfile::tempdir().unwrap();
    let site = directory.path().join("existing site");
    fs::create_dir(&site).unwrap();
    assert_success(&run(directory.path(), &["new", "existing site"]));
    assert_buildable_site(&site);
}

#[test]
fn generates_and_builds_a_site_in_the_empty_current_directory() {
    let directory = tempfile::tempdir().unwrap();
    assert_success(&run(directory.path(), &["new", "."]));
    assert_buildable_site(directory.path());
}

#[test]
fn refuses_nonempty_directories_without_changing_any_files() {
    for existing in ["index.md", ".keep", "layouts/default.html"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(existing);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "DO NOT OVERWRITE\r\n").unwrap();
        let before = tree(directory.path());
        let output = run(directory.path(), &["new", "."]);
        assert!(!output.status.success());
        assert_ne!(output.status.code(), Some(101));
        assert!(String::from_utf8_lossy(&output.stderr).contains("not empty"));
        assert_eq!(tree(directory.path()), before);
    }
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("empty-child")).unwrap();
    let output = run(directory.path(), &["new", "."]);
    assert!(!output.status.success());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn refuses_an_existing_file_as_the_destination() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("file"), "keep").unwrap();
    let output = run(directory.path(), &["new", "file"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a directory"));
    assert_eq!(
        fs::read_to_string(directory.path().join("file")).unwrap(),
        "keep"
    );
}

#[test]
fn validates_arguments_and_shows_help_without_creating_files() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["new", "--help"],
        vec!["new", "my-site", "--help"],
        vec!["new", "-h"],
    ] {
        let output = run(directory.path(), &args);
        assert_success(&output);
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(help.contains("<DIR>"));
        assert!(help.contains("上書き"));
        assert!(!help.contains("--force"));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
    for args in [
        vec!["new"],
        vec!["new", "site", "extra"],
        vec!["new", "site", "--force"],
        vec!["new", "site", "--unknown"],
    ] {
        let output = run(directory.path(), &args);
        assert!(!output.status.success());
        assert_ne!(output.status.code(), Some(101));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn refuses_symlink_destinations_including_broken_links() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), directory.path().join("linked-site")).unwrap();
    symlink(
        directory.path().join("missing"),
        directory.path().join("broken-site"),
    )
    .unwrap();
    for name in [
        "linked-site",
        "linked-site/",
        "linked-site/.",
        "broken-site",
    ] {
        let output = run(directory.path(), &["new", name]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("symbolic link"));
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert!(!directory.path().join("missing").exists());
}
