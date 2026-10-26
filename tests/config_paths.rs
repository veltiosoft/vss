use std::{
    fs,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn site() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "site/vss.toml", "");
    write(root.path(), "site/index.md", "# Home");
    write(root.path(), "site/layouts/default.html", "{{{contents}}}");
    write(root.path(), "site/static/important.txt", "static");
    write(root.path(), "site/partials/important.txt", "partial");
    write(root.path(), "site/.git/important.txt", "history");
    write(root.path(), "site/dist/keep.txt", "previous output");
    write(root.path(), "victim/important.txt", "outside");
    root
}

// A timeout also prevents a rejected serve configuration from leaving a server behind.
fn command(site: &Path, subcommand: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vss"))
        .arg(subcommand)
        .args(if subcommand == "serve" {
            vec!["--port", "0"]
        } else {
            vec![]
        })
        .current_dir(site)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() && start.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(10));
    }
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
        let _ = child.wait();
        panic!("{subcommand} did not reject invalid configuration");
    }
    child.wait_with_output().unwrap()
}

fn assert_preserved(root: &Path) {
    for (path, contents) in [
        ("site/index.md", "# Home"),
        ("site/layouts/default.html", "{{{contents}}}"),
        ("site/static/important.txt", "static"),
        ("site/partials/important.txt", "partial"),
        ("site/.git/important.txt", "history"),
        ("site/dist/keep.txt", "previous output"),
        ("victim/important.txt", "outside"),
    ] {
        assert_eq!(
            fs::read_to_string(root.join(path)).unwrap(),
            contents,
            "{path}"
        );
    }
}

#[test]
fn rejects_outside_dist_before_deleting_victim() {
    let root = site();
    write(root.path(), "site/vss.toml", "dist = '../victim'");
    let output = command(&root.path().join("site"), "build");
    assert!(!output.status.success());
    assert_preserved(root.path());
}

#[test]
fn rejects_unsafe_dist_before_deleting_any_files() {
    for subcommand in ["build", "serve"] {
        let root = site();
        let absolute = root.path().join("victim").display().to_string();
        for dist in [
            "",
            ".",
            "..",
            "../victim",
            "nested/../../victim",
            "/",
            &absolute,
            "//server/share",
            r"\\server\share",
            "C:/victim",
            r"C:\victim",
            "C:victim",
            "static",
            "STATIC",
            "static/output",
            "layouts",
            "LAYOUTS/output",
            "partials",
            "PARTIALS",
            ".git",
            ".git/output",
        ] {
            write(root.path(), "site/vss.toml", &format!("dist = '{}'", dist));
            let output = command(&root.path().join("site"), subcommand);
            assert!(!output.status.success(), "accepted {subcommand}: {dist}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("dist"),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_preserved(root.path());
        }
    }
}

#[test]
fn rejects_dist_overlap_with_custom_inputs_before_deleting() {
    for (static_dir, layouts, dist) in [
        ("assets", "templates", "assets/output"),
        ("assets", "templates", "templates"),
        ("source/assets", "source/templates", "source"),
    ] {
        let root = site();
        write(
            root.path(),
            "site/vss.toml",
            &format!("static = '{static_dir}'\nlayouts = '{layouts}'\ndist = '{dist}'"),
        );
        let output = command(&root.path().join("site"), "build");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("dist"));
        assert_preserved(root.path());
    }
}

#[test]
fn rejects_filesystem_aliases_of_inputs_before_deleting() {
    for dist in ["ässets", "ässets/output"] {
        let root = site();
        write(root.path(), "site/Ässets/keep.txt", "keep unicode input");
        // 大小文字を区別する FS では別名なので、この条件は対象外。
        if !root.path().join("site/ässets").exists() {
            continue;
        }
        write(
            root.path(),
            "site/vss.toml",
            &format!("static = 'Ässets'\ndist = '{dist}'"),
        );
        let output = command(&root.path().join("site"), "build");
        assert!(
            !output.status.success(),
            "accepted filesystem alias: {dist}"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("site/Ässets/keep.txt")).unwrap(),
            "keep unicode input"
        );
        assert_preserved(root.path());
    }
}

#[test]
fn rebuilds_dedicated_site_local_dist_directories() {
    for dist in ["dist", "public", "./public", "output/public"] {
        let root = site();
        write(root.path(), "site/vss.toml", &format!("dist = '{dist}'"));
        write(root.path(), &format!("site/{dist}/stale.txt"), "old");
        let output = command(&root.path().join("site"), "build");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!root.path().join(format!("site/{dist}/stale.txt")).exists());
        assert!(
            root.path()
                .join(format!("site/{dist}/index.html"))
                .is_file()
        );
    }
}

#[test]
fn rejects_outside_input_directories_before_deleting_outputs() {
    for setting in ["static", "layouts"] {
        for subcommand in ["build", "serve"] {
            let root = site();
            let absolute = root.path().join("victim").display().to_string();
            write(
                root.path(),
                "victim/default.html",
                "OUTSIDE TEMPLATE {{{contents}}}",
            );
            for value in [
                "../victim",
                &absolute,
                "",
                ".",
                "..",
                "//server/share",
                r"C:\victim",
                "C:victim",
                "dist",
                "dist/nested",
                ".git.",
                ".git ",
                ".ssh.",
                ".env.",
            ] {
                write(
                    root.path(),
                    "site/vss.toml",
                    &format!("{setting} = '{value}'"),
                );
                let output = command(&root.path().join("site"), subcommand);
                assert!(!output.status.success(), "accepted {setting} = {value:?}");
                assert_preserved(root.path());
                assert!(!root.path().join("site/dist/important.txt").exists());
            }
        }
    }
}

#[test]
fn excludes_private_static_paths_but_preserves_well_known() {
    let root = site();
    for path in [
        ".env",
        ".git/config",
        ".ssh/id_rsa",
        "nested/.env",
        "nested/.ssh/key",
        ".well-known/.env",
        ".well-known/.git/config",
        "nested/.well-known/secret",
    ] {
        write(
            root.path(),
            &format!("site/static/{path}"),
            "PRIVATE SENTINEL",
        );
    }
    write(
        root.path(),
        "site/static/.well-known/security.txt",
        "Contact: public",
    );
    write(
        root.path(),
        "site/static/.well-known/acme-challenge/token",
        "public token",
    );
    let output = command(&root.path().join("site"), "build");
    assert!(output.status.success());
    for path in [
        ".env",
        ".git",
        ".ssh",
        "nested/.env",
        "nested/.ssh",
        ".well-known/.env",
        ".well-known/.git",
        "nested/.well-known",
    ] {
        assert!(
            !root.path().join(format!("site/dist/{path}")).exists(),
            "published {path}"
        );
    }
    assert_eq!(
        fs::read_to_string(root.path().join("site/dist/.well-known/security.txt")).unwrap(),
        "Contact: public"
    );
    assert!(
        root.path()
            .join("site/dist/.well-known/acme-challenge/token")
            .is_file()
    );
}

#[cfg(unix)]
#[test]
fn rejects_dist_symlinks_and_symlink_ancestors() {
    for dist in ["link", "link/output", "link/missing/output"] {
        let root = site();
        std::os::unix::fs::symlink(root.path().join("victim"), root.path().join("site/link"))
            .unwrap();
        write(root.path(), "site/vss.toml", &format!("dist = '{dist}'"));
        let output = command(&root.path().join("site"), "build");
        assert!(!output.status.success());
        assert_preserved(root.path());
        assert!(root.path().join("site/link").is_symlink());
        assert!(!root.path().join("victim/output").exists());
        assert!(!root.path().join("victim/missing").exists());
    }
}
