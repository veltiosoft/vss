use super::{fixtures::*, *};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::Path, process::Command};

struct Fixture {
    release: Release,
    bodies: HashMap<String, Vec<u8>>,
    keys: Vec<self_update::VerifyingKey>,
}

fn fixture(target: &str, case: &str, contents: &[u8]) -> Fixture {
    let zip = target == "windows_amd64";
    let name = asset_name("2026.10.1", target).unwrap();
    let signer = key(11); // Reuse the existing public test-only seed; never a production key.
    let member = if case == "traversal" {
        "../outside"
    } else if zip {
        "vss.exe"
    } else {
        "vss"
    };
    let payload = unsigned_payload(zip, member, contents, case == "link", case == "extra-file");
    let context = match case {
        "wrong-version" => name.replace("2026.10.1", "2026.10.0"),
        "wrong-target" => name.replace(target, "another_target"),
        _ => name.clone(),
    };
    let mut bytes = match case {
        "unsigned" => payload,
        "invalid-container" => b"not an archive".to_vec(),
        _ => sign(&payload, zip, &context, std::slice::from_ref(&signer)),
    };
    if case == "tampered" {
        bytes[15] ^= 1;
    }
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let checksum = match case {
        "bad-checksum" => format!("{}  {name}\n", "0".repeat(64)).into_bytes(),
        "malformed-checksum" => b"invalid checksum".to_vec(),
        _ => format!("{hash}  {name}\n").into_bytes(),
    };
    let archive_url = format!("https://fixtures.invalid/{name}");
    let checksum_url = format!("{archive_url}.sha256");
    let mut assets = vec![
        serde_json::json!({"name": name, "url": archive_url, "digest": format!("sha256:{hash}")}),
        serde_json::json!({"name": format!("{name}.sha256"), "url": checksum_url}),
    ];
    match case {
        "missing-checksum" => {
            assets.pop();
        }
        "missing-archive" => {
            assets.remove(0);
        }
        "duplicate-archive" => assets.push(assets[0].clone()),
        "old-asset-name" => {
            assets[0]["name"] = serde_json::json!(format!(
                "vss_{target}.{}",
                if zip { "zip" } else { "tar.gz" }
            ))
        }
        "different-version-asset" => {
            assets[0]["name"] = serde_json::json!(name.replace("2026.10.1", "2026.10.0"))
        }
        "different-target-asset" => {
            assets[0]["name"] = serde_json::json!(name.replace(target, "another_target"))
        }
        "bad-digest" => assets[0]["digest"] = serde_json::json!("sha256:0000"),
        _ => {}
    }
    let tag = match case {
        "rollback" => "v2026.9.0",
        "equal" => "v2026.10.0",
        _ => "v2026.10.1",
    };
    let release = parse_release(
        &serde_json::to_vec(&serde_json::json!({"tag_name": tag, "assets": assets})).unwrap(),
    )
    .unwrap();
    let mut bodies = HashMap::from([
        (archive_url.clone(), bytes),
        (checksum_url.clone(), checksum),
    ]);
    if case == "checksum-download-error" {
        bodies.remove(&checksum_url);
    }
    if case == "archive-download-error" {
        bodies.remove(&archive_url);
    }
    let keys = match case {
        "empty-keys" => vec![],
        "unknown-key" => vec![key(12).verifying_key().to_bytes()],
        _ => vec![signer.verifying_key().to_bytes()],
    };
    Fixture {
        release,
        bodies,
        keys,
    }
}

impl Fixture {
    fn fetch(&self, asset: &ReleaseAsset, limit: u64, output: &mut dyn Write) -> Result<()> {
        let body = self
            .bodies
            .get(asset.download_url())
            .context("simulated download failure")?;
        ensure!(body.len() as u64 <= limit, "fixture exceeds download limit");
        output.write_all(body)?;
        Ok(())
    }
}

#[test]
fn complete_release_pipeline_accepts_all_targets_and_both_confirmation_modes() {
    for target in [
        "linux_amd64",
        "linux_arm64",
        "darwin_arm64",
        "windows_amd64",
    ] {
        for yes in [false, true] {
            let fixture = fixture(target, "valid", b"verified binary");
            let mut fetched = Vec::new();
            let mut installed = Vec::new();
            let mut approved = false;
            let outcome = apply_release(
                &fixture.release,
                "2026.10.0",
                target,
                &fixture.keys,
                |asset, limit, output| {
                    fetched.push((asset.name().to_owned(), limit));
                    fixture.fetch(asset, limit, output)
                },
                |summary| {
                    assert_eq!(summary.current, "2026.10.0");
                    assert_eq!(summary.version, "2026.10.1");
                    assert_eq!(summary.target, target);
                    assert_eq!(summary.name, asset_name("2026.10.1", target).unwrap());
                    approved = true;
                    confirm(yes, &mut &b"y\n"[..], &mut Vec::new())
                },
                |path| {
                    installed = fs::read(path)?;
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(outcome, UpdateOutcome::Updated("2026.10.1".into()));
            assert!(approved);
            assert_eq!(installed, b"verified binary");
            let name = asset_name("2026.10.1", target).unwrap();
            assert_eq!(
                fetched,
                [
                    (format!("{name}.sha256"), 4096),
                    (name, signed::MAX_ARCHIVE_SIZE)
                ]
            );
        }
    }
}

#[test]
fn release_failures_never_confirm_or_replace_and_fail_at_the_expected_gate() {
    for target in [
        "linux_amd64",
        "linux_arm64",
        "darwin_arm64",
        "windows_amd64",
    ] {
        for (case, fetch_count) in [
            ("empty-keys", 0),
            ("rollback", 0),
            ("missing-checksum", 0),
            ("missing-archive", 0),
            ("duplicate-archive", 0),
            ("old-asset-name", 0),
            ("different-version-asset", 0),
            ("different-target-asset", 0),
            ("malformed-checksum", 1),
            ("checksum-download-error", 1),
            ("archive-download-error", 2),
            ("bad-checksum", 2),
            ("bad-digest", 2),
            ("unknown-key", 2),
            ("unsigned", 2),
            ("tampered", 2),
            ("wrong-version", 2),
            ("wrong-target", 2),
            ("invalid-container", 2),
            ("traversal", 2),
            ("link", 2),
            ("extra-file", 2),
        ] {
            let fixture = fixture(target, case, b"untrusted binary");
            let mut fetched = 0;
            let error = apply_release(
                &fixture.release,
                "2026.10.0",
                target,
                &fixture.keys,
                |asset, limit, output| {
                    fetched += 1;
                    fixture.fetch(asset, limit, output)
                },
                |_| panic!("must not confirm {case} for {target}"),
                |_| panic!("must not replace {case} for {target}"),
            )
            .unwrap_err();
            assert_eq!(fetched, fetch_count, "{case} for {target}: {error:#}");
            if case == "invalid-container" {
                // Invalid bytes would fail in a decompressor if extraction ran
                // first. The error must come from the mandatory signature gate.
                assert!(format!("{error:#}").contains("署名検証"));
            }
        }
        let fixture = fixture(target, "equal", b"unused binary");
        assert_eq!(
            apply_release(
                &fixture.release,
                "2026.10.0",
                target,
                &fixture.keys,
                |_, _, _| panic!("equal version must not download"),
                |_| panic!("equal version must not confirm"),
                |_| panic!("equal version must not replace"),
            )
            .unwrap(),
            UpdateOutcome::UpToDate
        );
    }
}

const CHILD_TEST: &str = "subcommand_self::update::integration_tests::isolated_self_replace_child";
const CHILD_ROOT: &str = "VSS_TEST_ISOLATED_SELF_REPLACE_ROOT";

fn runner(root: &Path) -> std::path::PathBuf {
    root.join(format!("isolated-updater{}", std::env::consts::EXE_SUFFIX))
}

// This entry point only operates when invoked by the parent test from a copy of
// the libtest executable inside a marked sandbox. It cannot update real VSS or
// the cargo-owned test executable. No test-only switch exists in production.
#[test]
fn isolated_self_replace_child() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let root = std::path::PathBuf::from(root).canonicalize().unwrap();
    assert_eq!(
        fs::read(root.join("sandbox-marker")).unwrap(),
        b"vss isolated update test"
    );
    assert_eq!(
        std::env::current_exe().unwrap().canonicalize().unwrap(),
        runner(&root).canonicalize().unwrap()
    );
    let case = fs::read_to_string(root.join("case")).unwrap();
    let contents =
        fs::read(root.join(format!("successor{}", std::env::consts::EXE_SUFFIX))).unwrap();
    let target = get_target().unwrap();
    let fixture = fixture(target, &case, &contents);
    let outcome = apply_release(
        &fixture.release,
        "2026.10.0",
        target,
        &fixture.keys,
        |asset, limit, output| fixture.fetch(asset, limit, output),
        |_| {
            fs::write(root.join("approved"), b"verified before confirmation")?;
            confirm(case != "decline", &mut &b""[..], &mut Vec::new())
        },
        |path| {
            fs::write(root.join("install-called"), b"real self_replace")?;
            Ok(self_replace::self_replace(path)?)
        },
    );
    match case.as_str() {
        "valid" => assert_eq!(outcome.unwrap(), UpdateOutcome::Updated("2026.10.1".into())),
        "decline" => assert_eq!(outcome.unwrap(), UpdateOutcome::Cancelled),
        _ => {
            fs::write(root.join("error"), format!("{:#}", outcome.unwrap_err())).unwrap();
        }
    }
}

#[test]
fn real_self_replacement_in_a_copied_process_preserves_failures_and_runs_successor() {
    let root = tempfile::tempdir().unwrap();
    let successor = root
        .path()
        .join(format!("successor{}", std::env::consts::EXE_SUFFIX));
    let source = root.path().join("successor.rs");
    fs::write(
        &source,
        "fn main() { println!(\"signed update successor 2026.10.1\"); }\n",
    )
    .unwrap();
    let output = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name=vss_update_successor")
        .arg(&source)
        .arg("-o")
        .arg(&successor)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let original = std::env::current_exe().unwrap();
    let old_bytes = fs::read(&original).unwrap();
    for case in [
        "unsigned",
        "bad-checksum",
        "unknown-key",
        "wrong-version",
        "decline",
        "valid",
    ] {
        let sandbox = tempfile::tempdir().unwrap();
        let runner = runner(sandbox.path());
        fs::copy(&original, &runner).unwrap();
        fs::copy(
            &successor,
            sandbox.path().join(successor.file_name().unwrap()),
        )
        .unwrap();
        fs::write(
            sandbox.path().join("sandbox-marker"),
            b"vss isolated update test",
        )
        .unwrap();
        fs::write(sandbox.path().join("case"), case).unwrap();
        let output = Command::new(&runner)
            .args(["--exact", CHILD_TEST, "--nocapture"])
            .env(CHILD_ROOT, sandbox.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // A typo in the test filter must not turn the subprocess into a false pass.
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        if case == "valid" {
            assert!(sandbox.path().join("install-called").exists());
            assert_eq!(fs::read(&runner).unwrap(), fs::read(&successor).unwrap());
            let output = Command::new(&runner)
                .env_remove(CHILD_ROOT)
                .output()
                .unwrap();
            assert!(output.status.success());
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                "signed update successor 2026.10.1"
            );
        } else {
            assert_eq!(
                fs::read(&runner).unwrap(),
                old_bytes,
                "changed executable for {case}"
            );
            assert!(!sandbox.path().join("install-called").exists());
            assert_eq!(sandbox.path().join("approved").exists(), case == "decline");
            assert_eq!(sandbox.path().join("error").exists(), case != "decline");
        }
    }
    assert_eq!(
        fs::read(original).unwrap(),
        old_bytes,
        "cargo-owned test executable was modified"
    );
}
