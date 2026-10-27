use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

pub(super) const MAX_ARCHIVE_SIZE: u64 = 256 * 1024 * 1024;
const MAX_BINARY_SIZE: u64 = 256 * 1024 * 1024;

pub(super) fn require_keys(keys: &[self_update::VerifyingKey]) -> Result<()> {
    ensure!(
        !keys.is_empty(),
        "信頼済みの本番署名公開鍵が未設定のため自己更新を拒否しました。管理者による鍵の決定と安全な初回導入が必要です。docs/signed-updates.md を参照してください"
    );
    Ok(())
}

pub(super) fn checksum(sums: &[u8], name: &str) -> Result<String> {
    let text = std::str::from_utf8(sums).context("SHA-256 ファイルが UTF-8 ではありません")?;
    // Exactly one coreutils/shasum record, bound to the exact canonical asset.
    let line = text.strip_suffix('\n').unwrap_or(text);
    let line = line.strip_suffix('\r').unwrap_or(line);
    let (hash, file) = line
        .split_once("  ")
        .or_else(|| line.split_once(" *"))
        .context("不正な SHA-256 ファイル形式です")?;
    ensure!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && file == name,
        "SHA-256 は正規の成果物名と 64 桁のハッシュを 1 行だけ含む必要があります"
    );
    Ok(hash.to_ascii_lowercase())
}

fn sha256(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    io::copy(&mut File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn verify(
    path: &Path,
    name: &str,
    sums: &[u8],
    digest: Option<&str>,
    keys: &[self_update::VerifyingKey],
) -> Result<()> {
    // self_update::verify_signature intentionally accepts empty keys. Never call
    // it without our mandatory-key gate, and never fall back to unsigned assets.
    require_keys(keys)?;
    ensure!(
        path.file_name().and_then(|s| s.to_str()) == Some(name),
        "署名 context と成果物名が一致しません"
    );
    ensure!(
        fs::metadata(path)?.len() <= MAX_ARCHIVE_SIZE,
        "成果物がサイズ上限を超えています"
    );
    let expected = checksum(sums, name)?;
    let actual = sha256(path)?;
    ensure!(actual == expected, "SHA-256 が一致しません");
    if let Some(digest) = digest {
        let published = digest
            .strip_prefix("sha256:")
            .context("未対応の GitHub asset digest です")?;
        ensure!(
            published.len() == 64
                && published.bytes().all(|b| b.is_ascii_hexdigit())
                && published.eq_ignore_ascii_case(&actual),
            "GitHub asset digest が一致しません"
        );
    }
    self_update::verify_signature(path, keys).context("成果物の署名検証に失敗しました")?;
    Ok(())
}

/// Only this function reaches installation. The download is in a private temp
/// directory owned by the caller. No archive member determines an output path.
/// Verification must finish before staging; approval must finish before install.
pub(super) fn apply(
    path: &Path,
    name: &str,
    sums: &[u8],
    digest: Option<&str>,
    keys: &[self_update::VerifyingKey],
    approve: impl FnOnce() -> Result<bool>,
    install: impl FnOnce(&Path) -> Result<()>,
) -> Result<bool> {
    verify(path, name, sums, digest, keys)?;
    let stage = tempfile::tempdir()?;
    let binary_name = if name.ends_with(".zip") {
        "vss.exe"
    } else {
        "vss"
    };
    let binary = stage.path().join(binary_name);
    let mut output = File::options().write(true).create_new(true).open(&binary)?;
    extract(path, binary_name, &mut output)?;
    output.sync_all()?;
    drop(output);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))?;
    }
    if !approve()? {
        return Ok(false);
    }
    install(&binary)?;
    Ok(true)
}

fn copy_binary(input: &mut impl Read, output: &mut File) -> Result<()> {
    let copied = io::copy(&mut input.take(MAX_BINARY_SIZE + 1), output)?;
    ensure!(
        copied > 0 && copied <= MAX_BINARY_SIZE,
        "実行ファイルが空かサイズ上限を超えています"
    );
    Ok(())
}

fn extract(path: &Path, binary: &str, output: &mut File) -> Result<()> {
    if binary == "vss.exe" {
        let mut archive = zip::ZipArchive::new(File::open(path)?)?;
        ensure!(
            archive.len() == 1,
            "ZIP は通常ファイル vss.exe だけを含む必要があります"
        );
        let mut entry = archive.by_index(0)?;
        let kind = entry.unix_mode().unwrap_or(0o100000) & 0o170000;
        ensure!(
            entry.name_raw() == b"vss.exe" && !entry.is_dir() && matches!(kind, 0 | 0o100000),
            "ZIP に不要ファイル・リンク・不正なパスがあります"
        );
        ensure!(
            entry.size() <= MAX_BINARY_SIZE,
            "実行ファイルがサイズ上限を超えています"
        );
        copy_binary(&mut entry, output)?;
    } else {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(File::open(path)?));
        // Do not interpret GNU/PAX extended headers, links, or paths. The release
        // contract is one flat regular file, so no unpack()/extract_into().
        let mut entries = archive.entries()?.raw(true);
        let mut entry = entries.next().context("tar に vss がありません")??;
        ensure!(
            entry.path_bytes().as_ref() == b"vss" && entry.header().entry_type().is_file(),
            "tar に不要ファイル・リンク・不正なパスがあります"
        );
        ensure!(
            entry.size() <= MAX_BINARY_SIZE,
            "実行ファイルがサイズ上限を超えています"
        );
        copy_binary(&mut entry, output)?;
        drop(entry);
        ensure!(
            entries.next().is_none(),
            "tar は通常ファイル vss だけを含む必要があります"
        );
    }
    Ok(())
}

#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod tests;
