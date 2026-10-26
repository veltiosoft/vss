#[cfg(unix)]
mod unix {
    use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

    fn executable(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn install(platform: &str, corrupt: bool, extra_files: bool) {
        let root = tempfile::tempdir().unwrap();
        let fixture = root.path().join("fixture");
        let mocks = root.path().join("mocks");
        fs::create_dir_all(&fixture).unwrap();
        fs::create_dir_all(&mocks).unwrap();
        let binary = if platform == "Windows_NT" {
            "vss.exe"
        } else {
            "vss"
        };
        let archive = if platform == "Windows_NT" {
            "release.zip"
        } else {
            "release.tar.gz"
        };
        fs::write(fixture.join(binary), "verified binary").unwrap();
        fs::write(fixture.join("unexpected.txt"), "must not be extracted").unwrap();
        let mut pack = Command::new(if platform == "Windows_NT" {
            "zip"
        } else {
            "tar"
        });
        if platform == "Windows_NT" {
            pack.args(["-q", archive]);
        } else {
            pack.args(["czf", archive]);
        }
        pack.arg(binary);
        if extra_files {
            pack.arg("unexpected.txt");
        }
        assert!(pack.current_dir(&fixture).status().unwrap().success());
        let digest = Command::new("shasum")
            .args(["-a", "256", archive])
            .current_dir(&fixture)
            .output()
            .unwrap();
        let digest = String::from_utf8(digest.stdout).unwrap();
        let digest = if corrupt {
            "0".repeat(64)
        } else {
            digest.split_whitespace().next().unwrap().to_string()
        };
        let target = match platform {
            "Windows_NT" => "windows_amd64",
            "Darwin arm64" => "darwin_arm64",
            _ => "linux_amd64",
        };
        let extension = if platform == "Windows_NT" {
            "zip"
        } else {
            "tar.gz"
        };
        let asset = format!("vss_{target}.{extension}");
        fs::write(fixture.join("checksum"), format!("{digest}  {asset}\n")).unwrap();
        executable(
            &mocks.join("uname"),
            &format!("#!/bin/sh\necho '{platform}'\n"),
        );
        executable(
            &mocks.join("curl"),
            r#"#!/bin/sh
printf '%s\n' "$@" >> "$FIXTURE/curl-args"
output=
while [ "$#" -gt 0 ]; do
 case "$1" in
  --output|-o|-Lo) output=$2; shift 2 ;;
  https://*) url=$1; shift ;;
  *) shift ;;
 esac
done
case "$url" in
 *.sha256) cp "$FIXTURE/checksum" "$output" ;;
 *) cp "$FIXTURE/ARCHIVE" "$output" ;;
esac
"#
            .replace("ARCHIVE", archive)
            .as_str(),
        );
        let destination = root.path().join("install");
        fs::create_dir_all(destination.join("bin")).unwrap();
        fs::write(destination.join("bin").join(binary), "old binary").unwrap();
        let output = Command::new("sh")
            .arg(format!("{}/scripts/install.sh", env!("CARGO_MANIFEST_DIR")))
            .env(
                "PATH",
                format!("{}:{}", mocks.display(), std::env::var("PATH").unwrap()),
            )
            .env("VSS_INSTALL", &destination)
            .env("FIXTURE", &fixture)
            .env(
                "OS",
                if platform == "Windows_NT" {
                    "Windows_NT"
                } else {
                    ""
                },
            )
            .current_dir(root.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            !corrupt,
            "stdout: {} stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(destination.join("bin").join(binary)).unwrap(),
            if corrupt {
                "old binary"
            } else {
                "verified binary"
            }
        );
        assert!(!destination.join("bin/unexpected.txt").exists());
        if !corrupt {
            let args = fs::read_to_string(fixture.join("curl-args")).unwrap();
            assert!(
                args.contains("--proto\n=https\n")
                    && args.contains("--proto-redir\n=https\n")
                    && args.contains("--tlsv1.2")
            );
            assert!(args.contains(".sha256"));
        }
    }

    #[test]
    fn rejects_checksum_mismatch_without_replacing_existing_binary() {
        install("Linux x86_64", true, false);
    }

    #[test]
    fn installs_only_the_verified_binary_on_unix_and_windows_shells() {
        for platform in ["Linux x86_64", "Darwin arm64", "Windows_NT"] {
            install(platform, false, true);
        }
    }
}

#[cfg(windows)]
#[test]
fn powershell_installer_rejects_corruption_and_installs_flat_executable() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(root.join("tests/install_powershell.ps1"))
        .arg("-Installer")
        .arg(root.join("scripts/install.ps1"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn powershell_installer_uses_current_flat_release_and_verifies_before_installing() {
    let script = include_str!("../scripts/install.ps1");
    assert!(script.contains("https://github.com/veltiosoft/vss/releases/"));
    assert!(!script.contains("go-vss"));
    let verification = script.find("Get-FileHash").unwrap();
    let extraction = script.find("ZipFile]::OpenRead").unwrap();
    assert!(verification < extraction);
    assert!(script.contains("--proto-redir '=https'"));
    assert!(script.contains("GetEntry('vss.exe')"));
}
