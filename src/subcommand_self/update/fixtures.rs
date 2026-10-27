use std::io::{Cursor, Write};
use zipsign_api::SigningKey;

// Public, deterministic TEST-ONLY seeds. Not production trust roots/secrets.
pub(super) fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

pub(super) fn name(zip: bool) -> &'static str {
    if zip {
        "vss_2026.10.1_windows_amd64.zip"
    } else {
        "vss_2026.10.1_linux_amd64.tar.gz"
    }
}

pub(super) fn unsigned(zip: bool, member: &str, link: bool, extra: bool) -> Vec<u8> {
    unsigned_payload(zip, member, b"verified binary", link, extra)
}

pub(super) fn unsigned_payload(
    zip: bool,
    member: &str,
    contents: &[u8],
    link: bool,
    extra: bool,
) -> Vec<u8> {
    if zip {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        if link {
            writer
                .add_symlink(member, "../../outside", options)
                .unwrap();
        } else {
            writer.start_file(member, options).unwrap();
            writer.write_all(contents).unwrap();
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
        let data: &[u8] = if link { b"" } else { contents };
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

pub(super) fn sign(bytes: &[u8], zip: bool, context: &str, signers: &[SigningKey]) -> Vec<u8> {
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
