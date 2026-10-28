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
    assert_eq!(fs::read(directory.join("dist/.gitignore")).unwrap(), b"*\n");
    assert!(!directory.join(".gitignore").exists());
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
fn preserves_unrelated_files_and_reuses_existing_directories() {
    let directory = tempfile::tempdir().unwrap();
    for name in [
        ".git/config",
        ".gitignore",
        ".keep",
        "notes.txt",
        "layouts/custom.html",
        "layouts/tags/custom.html",
        "static/custom.css",
    ] {
        let path = directory.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "KEEP\r\n").unwrap();
    }
    fs::create_dir(directory.path().join("empty-child")).unwrap();
    let before = tree(directory.path());
    assert_success(&run(directory.path(), &["new", "."]));
    for (name, contents) in before {
        assert_eq!(fs::read(directory.path().join(name)).unwrap(), contents);
    }
    assert!(directory.path().join("empty-child").is_dir());
    assert_success(&run(directory.path(), &["build"]));
    assert!(directory.path().join("dist/index.html").is_file());
    assert_eq!(
        fs::read(directory.path().join(".gitignore")).unwrap(),
        b"KEEP\r\n"
    );
}

#[test]
fn accepts_a_git_worktree_metadata_file() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join(".git"),
        "gitdir: ../repository/worktrees/site\n",
    )
    .unwrap();
    assert_success(&run(directory.path(), &["new", "."]));
    assert_eq!(
        fs::read_to_string(directory.path().join(".git")).unwrap(),
        "gitdir: ../repository/worktrees/site\n"
    );
}

#[test]
fn refuses_collisions_before_creating_any_skeleton_files_or_directories() {
    for existing in [
        "vss.toml",
        "index.md",
        "layouts/default.html",
        "layouts/tags/default.html",
        "static/style.css",
        "README.md",
        "layouts",
        "layouts/tags",
        "static",
    ] {
        for is_directory in [false, true] {
            if is_directory && ["layouts", "layouts/tags", "static"].contains(&existing) {
                continue;
            }
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(existing);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if is_directory {
                fs::create_dir(path).unwrap();
            } else {
                fs::write(path, "DO NOT OVERWRITE\r\n").unwrap();
            }
            fs::write(directory.path().join(".gitignore"), "keep\n").unwrap();
            let before = tree(directory.path());
            let before_entries = WalkDir::new(directory.path()).into_iter().count();
            let output = run(directory.path(), &["new", "."]);
            assert!(!output.status.success(), "accepted collision: {existing}");
            assert_ne!(output.status.code(), Some(101));
            assert!(String::from_utf8_lossy(&output.stderr).contains("Destination"));
            assert_eq!(tree(directory.path()), before);
            assert_eq!(
                WalkDir::new(directory.path()).into_iter().count(),
                before_entries
            );
        }
    }
}

#[test]
fn rerunning_new_preserves_the_existing_site() {
    let directory = tempfile::tempdir().unwrap();
    assert_success(&run(directory.path(), &["new", "."]));
    let before = tree(directory.path());
    let output = run(directory.path(), &["new", "."]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert_eq!(tree(directory.path()), before);
}

#[test]
fn output_gitignore_excludes_all_generated_files_in_a_git_repository() {
    let directory = tempfile::tempdir().unwrap();
    assert_success(
        &Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(directory.path())
            .output()
            .unwrap(),
    );
    assert_success(&run(directory.path(), &["new", "."]));
    for dist in ["dist", "public", "output/public"] {
        fs::write(
            directory.path().join("vss.toml"),
            format!("dist = '{dist}'"),
        )
        .unwrap();
        for _ in 0..2 {
            assert_success(&run(directory.path(), &["build"]));
            let paths =
                [".gitignore", "index.html", "style.css"].map(|name| format!("{dist}/{name}"));
            let output = Command::new("git")
                .args(["check-ignore", "--"])
                .args(&paths)
                .current_dir(directory.path())
                .output()
                .unwrap();
            assert_success(&output);
            assert_eq!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .lines()
                    .collect::<Vec<_>>(),
                paths
            );
            assert!(!directory.path().join(".gitignore").exists());
        }
    }
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
fn rejects_symlinks_in_skeleton_paths_before_creating_files() {
    use std::os::unix::fs::symlink;
    for name in [
        "layouts",
        "layouts/tags",
        "static",
        "README.md",
        "layouts/default.html",
    ] {
        for broken in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            let path = directory.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let target = if broken {
                outside.path().join("missing")
            } else {
                outside.path().to_path_buf()
            };
            symlink(&target, &path).unwrap();
            let before_entries = WalkDir::new(directory.path()).into_iter().count();
            let output = run(directory.path(), &["new", "."]);
            assert!(!output.status.success(), "accepted symlink: {name}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("symbolic link"));
            assert!(tree(directory.path()).is_empty());
            assert_eq!(
                WalkDir::new(directory.path()).into_iter().count(),
                before_entries
            );
            assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
            assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        }
    }
}

#[cfg(unix)]
#[test]
fn preserves_unrelated_symlinks() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), directory.path().join("unrelated")).unwrap();
    symlink(
        outside.path().join("missing"),
        directory.path().join(".gitignore"),
    )
    .unwrap();
    assert_success(&run(directory.path(), &["new", "."]));
    for name in ["unrelated", ".gitignore"] {
        assert!(
            fs::symlink_metadata(directory.path().join(name))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn refuses_symlink_ancestors_without_creating_files_in_the_target() {
    use std::os::unix::fs::symlink;
    for destination in ["linked/site", "linked/missing/site", "broken/site"] {
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), directory.path().join("linked")).unwrap();
        symlink(
            directory.path().join("missing"),
            directory.path().join("broken"),
        )
        .unwrap();
        let output = run(directory.path(), &["new", destination]);
        assert!(!output.status.success(), "accepted {destination}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("symbolic link"));
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        assert!(!directory.path().join("missing").exists());
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
