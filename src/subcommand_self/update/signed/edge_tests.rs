use super::super::fixtures::{key, name, sign, unsigned_payload};
use super::*;

fn rejects_signed_payload(payload: &[u8], zip: bool, expected_error: &str) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(name(zip));
    let signer = key(11);
    fs::write(
        &path,
        sign(payload, zip, name(zip), std::slice::from_ref(&signer)),
    )
    .unwrap();
    let sums = format!("{}  {}\n", sha256(&path).unwrap(), name(zip));
    let error = apply(
        &path,
        name(zip),
        sums.as_bytes(),
        None,
        &[signer.verifying_key().to_bytes()],
        || panic!("unsafe archive must not reach approval"),
        |_| panic!("unsafe archive must not reach replacement"),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains(expected_error), "{error:#}");
}

#[test]
fn trusted_empty_executables_are_rejected_in_both_formats() {
    for zip in [false, true] {
        let payload = unsigned_payload(zip, if zip { "vss.exe" } else { "vss" }, b"", false, false);
        rejects_signed_payload(&payload, zip, "実行ファイルが空");
    }
}

#[test]
fn trusted_tar_hardlinks_directories_devices_and_extensions_are_rejected() {
    for kind in *b"15346xgLS" {
        let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(gzip);
        let mut header = tar::Header::new_gnu();
        header.set_path("vss").unwrap();
        header.set_entry_type(tar::EntryType::new(kind));
        header.set_link_name("../../outside").unwrap();
        header.set_size(0);
        header.set_mode(0o755);
        header.set_cksum();
        archive.append(&header, &b""[..]).unwrap();
        let payload = archive.into_inner().unwrap().finish().unwrap();
        rejects_signed_payload(&payload, false, "tar に不要ファイル");
    }
}

#[test]
fn oversized_download_is_rejected_before_hashing_or_archive_parsing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(name(false));
    // Sparse file: exercise the size gate without allocating/hashing 256 MiB.
    File::create(&path)
        .unwrap()
        .set_len(MAX_ARCHIVE_SIZE + 1)
        .unwrap();
    let error = apply(
        &path,
        name(false),
        b"not even a checksum",
        None,
        &[key(11).verifying_key().to_bytes()],
        || panic!("oversized download must not reach approval"),
        |_| panic!("oversized download must not reach replacement"),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("成果物がサイズ上限"));
}
