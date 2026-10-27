use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Workflow {
    jobs: HashMap<String, Job>,
    permissions: Option<HashMap<String, String>>,
}
#[derive(Deserialize)]
struct Job {
    steps: Vec<Step>,
    permissions: Option<HashMap<String, String>>,
}
#[derive(Deserialize)]
struct Step {
    name: Option<String>,
    run: Option<String>,
    uses: Option<String>,
}

fn workflow() -> Workflow {
    serde_saphyr::from_str(include_str!("../.github/workflows/release.yaml")).unwrap()
}

#[test]
fn release_actions_are_pinned_and_permissions_are_scoped() {
    let workflow = workflow();
    assert_eq!(
        workflow
            .permissions
            .unwrap()
            .get("contents")
            .map(String::as_str),
        Some("read")
    );
    for job in workflow.jobs.values() {
        for step in &job.steps {
            if let Some(action) = &step.uses {
                let (_, revision) = action.rsplit_once('@').unwrap();
                assert!(
                    revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()),
                    "unpinned {action}"
                );
            }
        }
    }
    for name in ["github-release", "build", "publish"] {
        assert_eq!(
            workflow.jobs[name]
                .permissions
                .as_ref()
                .unwrap()
                .get("contents")
                .map(String::as_str),
            Some("write")
        );
    }
}

#[cfg(unix)]
#[test]
fn release_commands_do_not_interpret_tag_names_as_shell_code() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    let workflow = workflow();
    for job in workflow.jobs.values() {
        for step in &job.steps {
            let Some(script) = &step.run else { continue };
            if !script.contains("gh release") {
                continue;
            }
            for tag in [
                "v1.2.3$(touch${IFS}injected)",
                "v1.2.3`touch${IFS}injected`",
                "v1.2.3;touch injected;#",
                "v1.2.3",
            ] {
                let directory = tempfile::tempdir().unwrap();
                let mock = directory.path().join("gh");
                fs::write(&mock, "#!/bin/sh\nprintf '%s\\n' \"$@\" > gh-args\n").unwrap();
                fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();
                let script = script
                    .replace("${{ steps.get_version.outputs.VERSION }}", tag)
                    .replace("${{ needs.github-release.outputs.version }}", tag);
                let output = Command::new("bash")
                    .args(["-e", "-c", &script])
                    .current_dir(directory.path())
                    .env(
                        "PATH",
                        format!(
                            "{}:{}",
                            directory.path().display(),
                            std::env::var("PATH").unwrap()
                        ),
                    )
                    .env("VERSION", tag)
                    .env("GITHUB_REPOSITORY", "owner/repo")
                    .env("ARCHIVE", "vss_linux_amd64.tar.gz")
                    .output()
                    .unwrap();
                assert!(
                    !directory.path().join("injected").exists(),
                    "shell injection in {:?}",
                    step.name
                );
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let args = fs::read_to_string(directory.path().join("gh-args")).unwrap();
                assert!(
                    args.lines().any(|arg| arg == tag),
                    "tag was not passed literally: {args}"
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn release_build_generates_checksum_and_attestation_before_upload() {
    use std::{fs, process::Command};
    let workflow = workflow();
    let job = &workflow.jobs["build"];
    let steps = &job.steps;
    let checksum = steps
        .iter()
        .position(|step| step.name.as_deref() == Some("Generate checksum"))
        .unwrap();
    let attestation = steps
        .iter()
        .position(|step| {
            step.uses
                .as_ref()
                .is_some_and(|action| action.starts_with("actions/attest-build-provenance@"))
        })
        .unwrap();
    let upload = steps
        .iter()
        .position(|step| step.name.as_deref() == Some("Upload Release Asset"))
        .unwrap();
    assert!(checksum < attestation && attestation < upload);
    for permission in ["id-token", "attestations"] {
        assert_eq!(job.permissions.as_ref().unwrap()[permission], "write");
    }
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("archive.tar.gz"), "test").unwrap();
    let output = Command::new("bash")
        .args(["-e", "-c", steps[checksum].run.as_ref().unwrap()])
        .current_dir(directory.path())
        .env("ARCHIVE", "archive.tar.gz")
        .output()
        .unwrap();
    assert!(output.status.success());
    let checksum = fs::read_to_string(directory.path().join("archive.tar.gz.sha256")).unwrap();
    assert_eq!(
        checksum.split_whitespace().next().unwrap(),
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    );
    assert!(
        steps[upload]
            .run
            .as_ref()
            .unwrap()
            .contains("\"$ARCHIVE.sha256\"")
    );
}

#[cfg(unix)]
#[test]
fn version_step_rejects_malicious_tags_and_accepts_release_tags() {
    use std::{fs, process::Command};
    let workflow = workflow();
    let script = workflow.jobs["github-release"]
        .steps
        .iter()
        .find(|step| step.name.as_deref() == Some("Get the version"))
        .unwrap()
        .run
        .as_ref()
        .unwrap();
    for (tag, accepted) in [
        // CalVer YYYY.M.MICRO (https://calver.org): the major is the full
        // four-digit year and the month segment runs 1-12, never zero-padded.
        ("v2026.10.0", true),
        ("2026.10.0", true),
        ("v2026.10.1", true),
        ("v2026.10.0-rc.1", true),
        ("v26.10.0", false),
        ("v2026.13.0", false),
        ("v2026.010.0", false),
        ("v2026.10.0.1", false),
        ("v2026.10.0$(id)", false),
        ("v2026.10.0`id`", false),
        ("v2026.10.0;id", false),
        ("--help", false),
        ("", false),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output_file = directory.path().join("output");
        let output = Command::new("bash")
            .args(["-e", "-c", script])
            .current_dir(directory.path())
            .env("GITHUB_REF_NAME", tag)
            .env("GITHUB_REF", format!("refs/tags/{tag}"))
            .env("GITHUB_OUTPUT", &output_file)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), accepted, "tag {tag:?}");
        if accepted {
            assert_eq!(
                fs::read_to_string(output_file).unwrap().trim(),
                format!("VERSION={tag}")
            );
        } else {
            assert!(!output_file.exists());
        }
    }
}

#[cfg(unix)]
#[test]
fn release_tag_must_match_package_before_draft_creation_and_build_is_locked() {
    use std::{fs, process::Command};
    let workflow = workflow();
    let steps = &workflow.jobs["github-release"].steps;
    let check = steps
        .iter()
        .position(|step| step.name.as_deref() == Some("Verify tag matches Cargo package version"))
        .unwrap();
    let create = steps
        .iter()
        .position(|step| step.name.as_deref() == Some("Create draft release"))
        .unwrap();
    assert!(check < create);
    for (tag, package, accepted) in [
        ("v2026.10.0", "2026.10.0", true),
        ("2026.10.0", "2026.10.0", true),
        ("v2026.10.0-rc.1+build.2", "2026.10.0-rc.1+build.2", true),
        ("v2026.10.1", "2026.10.0", false),
        ("vv2026.10.0", "2026.10.0", false),
        ("v2026.10.0+build.2", "2026.10.0+build.1", false),
        ("v2026.10.0-rc.1", "2026.10.0", false),
        ("v2026.10.0$(touch injected)", "2026.10.0", false),
    ] {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join("Cargo.toml"),
            format!("[package]\nversion = \"{package}\"\n"),
        )
        .unwrap();
        let output = Command::new("bash")
            .args(["-e", "-c", steps[check].run.as_ref().unwrap()])
            .current_dir(directory.path())
            .env("VERSION", tag)
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            accepted,
            "{tag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!directory.path().join("injected").exists());
        if !accepted {
            assert!(String::from_utf8_lossy(&output.stderr).contains("does not match"));
        }
    }
    let build = workflow.jobs["build"]
        .steps
        .iter()
        .find(|step| step.name.as_deref() == Some("Build"))
        .unwrap()
        .run
        .as_ref()
        .unwrap();
    assert!(build.split_whitespace().any(|arg| arg == "--locked"));
}
