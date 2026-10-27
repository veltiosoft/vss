use super::*;
use std::{
    cell::Cell,
    io::{Cursor, Write},
    path::PathBuf,
};
use zipsign_api::SigningKey;

// Public, deterministic TEST-ONLY seeds. Not production trust roots/secrets.
fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn name(zip: bool) -> &'static str {
    if zip {
        "vss_2026.10.1_windows_amd64.zip"
    } else {
        "vss_2026.10.1_linux_amd64.tar.gz"
    }
}

fn unsigned(zip: bool, member: &str, link: bool, extra: bool) -> Vec<u8> {
    if zip {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        if link {
            writer
                .add_symlink(member, "../../outside", options)
                .unwrap();
        } else {
            writer.start_file(member, options).unwrap();
            writer.write_all(b"verified binary").unwrap();
        }
        if extra {
            writer.start_file("unexpected.txt", options).unwrap();
            writer.write_all(b"must not be extracted").unwrap();
        }
        writer.finish().unwrap().into_inner()
    } else {
        let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut writer = tar::Builder::new(gzip);
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        let data: &[u8] = if link { b"" } else { b"verified binary" };
        header.set_size(data.len() as u64);
        if link {
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_link_name("../../outside").unwrap();
        }
        // Raw name permits traversal/absolute path fixtures which append_data rejects.
        header.as_mut_bytes()[..100].fill(0);
        header.as_mut_bytes()[..member.len()].copy_from_slice(member.as_bytes());
        header.set_cksum();
        writer.append(&header, data).unwrap();
        if extra {
            writer.append(&header, data).unwrap();
        }
        writer.into_inner().unwrap().finish().unwrap()
    }
}

fn sign(bytes: &[u8], zip: bool, context: &str, signers: &[SigningKey]) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    if zip {
        zipsign_api::sign::copy_and_sign_zip(
            &mut Cursor::new(bytes.to_vec()),
            &mut output,
            signers,
            Some(context.as_bytes()),
        )
        .unwrap();
    } else {
        zipsign_api::sign::copy_and_sign_tar(
            &mut Cursor::new(bytes),
            &mut output,
            signers,
            Some(context.as_bytes()),
        )
        .unwrap();
    }
    output.into_inner()
}

fn sums(path: &Path, name: &str) -> Vec<u8> {
    format!("{}  {name}\n", sha256(path).unwrap()).into_bytes()
}

struct Fixture {
    _root: tempfile::TempDir,
    archive: PathBuf,
    destination: PathBuf,
    outside: PathBuf,
}

impl Fixture {
    fn new(zip: bool, bytes: &[u8]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join(name(zip));
        let destination = root.path().join("installed-vss");
        let outside = root.path().join("outside");
        fs::write(&archive, bytes).unwrap();
        fs::write(&destination, "old binary").unwrap();
        Self {
            _root: root,
            archive,
            destination,
            outside,
        }
    }

    fn apply(
        &self,
        checksum: &[u8],
        digest: Option<&str>,
        keys: &[self_update::VerifyingKey],
        approve: impl FnOnce() -> Result<bool>,
    ) -> Result<bool> {
        super::apply(
            &self.archive,
            self.archive.file_name().unwrap().to_str().unwrap(),
            checksum,
            digest,
            keys,
            approve,
            |binary| {
                fs::copy(binary, &self.destination)?;
                Ok(())
            },
        )
    }

    fn unchanged(&self) {
        assert_eq!(fs::read(&self.destination).unwrap(), b"old binary");
        assert!(!self.outside.exists());
        assert!(!self._root.path().join("unexpected.txt").exists());
        assert!(!self._root.path().join("vss").exists());
        assert!(!self._root.path().join("vss.exe").exists());
    }
}

#[test]
fn signed_tar_and_zip_with_sha256_can_install_and_decline_safely() {
    for zip in [false, true] {
        let signer = key(11);
        let bytes = sign(
            &unsigned(zip, if zip { "vss.exe" } else { "vss" }, false, false),
            zip,
            name(zip),
            std::slice::from_ref(&signer),
        );
        let fixture = Fixture::new(zip, &bytes);
        let checksum = sums(&fixture.archive, name(zip));
        let keys = [signer.verifying_key().to_bytes()];
        assert!(!fixture.apply(&checksum, None, &keys, || Ok(false)).unwrap());
        fixture.unchanged();
        let digest = format!("sha256:{}", sha256(&fixture.archive).unwrap());
        assert!(
            fixture
                .apply(&checksum, Some(&digest), &keys, || Ok(true))
                .unwrap()
        );
        assert_eq!(fs::read(&fixture.destination).unwrap(), b"verified binary");
    }
}

#[test]
fn invalid_downloads_never_reach_staging_confirmation_or_replacement_even_with_yes() {
    for zip in [false, true] {
        for case in [
            "unsigned",
            "unknown-key",
            "empty-keys",
            "tampered",
            "signature-corrupt",
            "wrong-version",
            "wrong-target",
            "renamed",
            "missing-checksum",
            "malformed-checksum",
            "wrong-checksum",
            "wrong-checksum-name",
            "duplicate-checksum",
            "bad-digest",
        ] {
            let signer = key(11);
            let payload = unsigned(zip, if zip { "vss.exe" } else { "vss" }, false, false);
            let mut bytes = if case == "unsigned" {
                payload.clone()
            } else {
                let context = match case {
                    "wrong-version" => name(zip).replace("2026.10.1", "2026.10.0"),
                    "wrong-target" => {
                        name(zip).replace(if zip { "windows" } else { "linux" }, "darwin")
                    }
                    "renamed" => format!("other_{}", name(zip)),
                    _ => name(zip).to_owned(),
                };
                sign(&payload, zip, &context, std::slice::from_ref(&signer))
            };
            if case == "tampered" {
                bytes[15] ^= 1;
            }
            if case == "signature-corrupt" {
                let last = bytes.len() - 1;
                bytes[last] ^= 1;
            }
            let fixture = Fixture::new(zip, &bytes);
            let correct = sums(&fixture.archive, name(zip));
            let checksum = match case {
                "missing-checksum" => Vec::new(),
                "malformed-checksum" => format!("xyz  {}\n", name(zip)).into_bytes(),
                "wrong-checksum" => format!("{}  {}\n", "0".repeat(64), name(zip)).into_bytes(),
                "wrong-checksum-name" => correct
                    .iter()
                    .copied()
                    .chain(b"extra".iter().copied())
                    .collect(),
                "duplicate-checksum" => [correct.clone(), correct.clone()].concat(),
                _ => correct,
            };
            let keys = match case {
                "empty-keys" => vec![],
                "unknown-key" => vec![key(12).verifying_key().to_bytes()],
                _ => vec![signer.verifying_key().to_bytes()],
            };
            let digest = (case == "bad-digest").then_some("sha256:0000");
            let confirmed = Cell::new(false);
            let result = fixture.apply(&checksum, digest, &keys, || {
                confirmed.set(true);
                // This is the same --yes confirmation function used by the CLI.
                super::super::confirm(true, &mut &b""[..], &mut Vec::new())
            });
            assert!(result.is_err(), "{case}, zip={zip}");
            assert!(!confirmed.get(), "approved {case}, zip={zip}");
            fixture.unchanged();
            // All these errors are in verification, not an extraction/parser error.
            let error = format!("{:#}", result.unwrap_err());
            assert!(
                error.contains("署名") || error.contains("SHA-256") || error.contains("digest"),
                "{case}: {error}"
            );
        }
    }
}

#[test]
fn even_trusted_archives_cannot_install_links_traversal_or_extra_files() {
    for zip in [false, true] {
        let binary = if zip { "vss.exe" } else { "vss" };
        for (member, link, extra) in [
            (binary, true, false),
            (binary, false, true),
            ("../outside", false, false),
            ("/outside", false, false),
            ("dir/vss", false, false),
            ("..\\outside", false, false),
            ("unwanted", false, false),
        ] {
            let signer = key(11);
            let payload = unsigned(zip, member, link, extra);
            let bytes = sign(&payload, zip, name(zip), std::slice::from_ref(&signer));
            let fixture = Fixture::new(zip, &bytes);
            let result = fixture.apply(
                &sums(&fixture.archive, name(zip)),
                None,
                &[signer.verifying_key().to_bytes()],
                || panic!("must reject structure before confirmation"),
            );
            assert!(result.is_err(), "{member}, zip={zip}");
            fixture.unchanged();
        }
    }
}

#[test]
fn rotation_accepts_either_trusted_key_but_removing_old_key_rejects_old_only() {
    for zip in [false, true] {
        let old = key(11);
        let new = key(12);
        let old_pub = old.verifying_key().to_bytes();
        let new_pub = new.verifying_key().to_bytes();
        let payload = unsigned(zip, if zip { "vss.exe" } else { "vss" }, false, false);
        for signers in [
            vec![old.clone()],
            vec![new.clone()],
            vec![old.clone(), new.clone()],
        ] {
            let fixture = Fixture::new(zip, &sign(&payload, zip, name(zip), &signers));
            let checksum = sums(&fixture.archive, name(zip));
            assert!(
                fixture
                    .apply(&checksum, None, &[old_pub, new_pub], || Ok(true))
                    .unwrap()
            );
            if signers.len() == 2 {
                for trusted in [old_pub, new_pub] {
                    assert!(
                        fixture
                            .apply(&checksum, None, &[trusted], || Ok(true))
                            .unwrap()
                    );
                }
            }
        }
        let fixture = Fixture::new(zip, &sign(&payload, zip, name(zip), &[old]));
        assert!(
            fixture
                .apply(
                    &sums(&fixture.archive, name(zip)),
                    None,
                    &[new_pub],
                    || panic!("old key removed")
                )
                .is_err()
        );
        fixture.unchanged();
    }
}

#[test]
fn checksum_format_is_strict_and_binary_mode_is_supported() {
    let name = "vss_2026.10.1_linux_amd64.tar.gz";
    let hash = "a".repeat(64);
    for record in [format!("{hash}  {name}\n"), format!("{hash} *{name}\r\n")] {
        assert_eq!(checksum(record.as_bytes(), name).unwrap(), hash);
    }
    for record in [
        hash.clone(),
        format!("{hash}  dist/{name}\n"),
        format!("{hash}  {name}\n\n"),
        format!("{}  {name}\n", "g".repeat(64)),
        format!("{hash} {name}\n"),
    ] {
        assert!(checksum(record.as_bytes(), name).is_err());
    }
}
