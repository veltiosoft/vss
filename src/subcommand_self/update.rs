mod signed;

use anyhow::{Context, Result, bail, ensure};
use self_update::update::{Release, ReleaseAsset};
use std::{
    fs::File,
    io::{self, BufRead, IsTerminal, Read, Write},
    time::Duration,
};

const REPO_OWNER: &str = "veltiosoft";
const REPO_NAME: &str = "vss";

// Intentionally empty until the administrator approves production trust roots.
// Never add test keys or load replacement trust roots from the network/environment.
// See docs/signed-updates.md for the release/installer contract and blockers.
const TRUSTED_KEYS: &[self_update::VerifyingKey] = &[];

const YES_FLAG: noargs::FlagSpec =
    noargs::flag("yes").doc("確認操作だけを省略します（署名・SHA-256 検証は必須）");

pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let yes = YES_FLAG.take(&mut args).is_present();
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }
    update(yes).map_err(|error| format!("更新に失敗しました: {error:#}"))?;
    Ok(())
}

fn require_confirmation_mode(yes: bool, terminal: bool) -> Result<()> {
    ensure!(
        yes || terminal,
        "非対話環境では vss self update --yes が必要です（検証は省略されません）"
    );
    Ok(())
}

fn update(yes: bool) -> Result<()> {
    require_confirmation_mode(yes, io::stdin().is_terminal() && io::stdout().is_terminal())?;
    let target = get_target()?;
    signed::require_keys(TRUSTED_KEYS)?; // Fail before network access, even with --yes.
    let current = env!("CARGO_PKG_VERSION");
    println!("最新バージョンを確認しています...");
    // Fetch exactly once and preserve the raw tag until validated. The backend's
    // get_latest_release() strips *all* leading v characters before exposing it.
    let release = latest_release()?;
    let outcome = apply_release(
        &release,
        current,
        target,
        TRUSTED_KEYS,
        download,
        |summary| {
            println!(
                "現在: {}\n更新先: {}\n対象: {}\n成果物: {}",
                summary.current, summary.version, summary.target, summary.name
            );
            println!("✓ 信頼済み鍵の署名・SHA-256・アーカイブ構造の検証に成功しました");
            confirm(yes, &mut io::stdin().lock(), &mut io::stdout().lock())
        },
        |binary| Ok(self_replace::self_replace(binary)?),
    )?;
    match outcome {
        UpdateOutcome::UpToDate => println!("✓ すでに最新バージョン ({current}) です"),
        UpdateOutcome::Cancelled => println!("更新をキャンセルしました。"),
        UpdateOutcome::Updated(version) => {
            println!("✓ vss を {version} に更新しました。コマンドを再実行してください。")
        }
    }
    Ok(())
}

struct UpdateSummary<'a> {
    current: &'a str,
    version: String,
    target: &'a str,
    name: String,
}

#[derive(Debug, PartialEq)]
enum UpdateOutcome {
    UpToDate,
    Cancelled,
    Updated(String),
}

// Private dependency injection permits offline tests of the entire selection →
// download → verification → confirmation → replacement path. No runtime trust,
// URL override, or verification bypass is exposed by the production CLI.
fn apply_release(
    release: &Release,
    current: &str,
    target: &str,
    keys: &[self_update::VerifyingKey],
    mut fetch: impl FnMut(&ReleaseAsset, u64, &mut dyn Write) -> Result<()>,
    approve: impl FnOnce(&UpdateSummary<'_>) -> Result<bool>,
    install: impl FnOnce(&std::path::Path) -> Result<()>,
) -> Result<UpdateOutcome> {
    signed::require_keys(keys)?;
    let version = canonical_version(release.version())?;
    if !is_newer(current, &version)? {
        return Ok(UpdateOutcome::UpToDate);
    }
    let name = asset_name(&version, target)?;
    let archive_asset = exact_asset(release, &name)?;
    let checksum_asset = exact_asset(release, &format!("{name}.sha256"))?;
    let mut checksum = Vec::new();
    fetch(checksum_asset, 4096, &mut checksum)?;
    signed::checksum(&checksum, &name)?; // Reject malformed sums before pulling the archive.
    let download_dir = tempfile::tempdir()?;
    let path = download_dir.path().join(&name);
    {
        let mut file = File::create(&path)?;
        fetch(archive_asset, signed::MAX_ARCHIVE_SIZE, &mut file)?;
        file.sync_all()?;
    }
    let summary = UpdateSummary {
        current,
        version,
        target,
        name,
    };
    let installed = signed::apply(
        &path,
        &summary.name,
        &checksum,
        archive_asset.digest(),
        keys,
        || approve(&summary),
        install,
    )?;
    Ok(if installed {
        UpdateOutcome::Updated(summary.version)
    } else {
        UpdateOutcome::Cancelled
    })
}

fn confirm(yes: bool, input: &mut impl io::BufRead, output: &mut impl Write) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    write!(output, "この成果物で更新しますか？ [y/N] ")?;
    output.flush()?;
    let mut answer = String::new();
    let read = input.take(16).read_line(&mut answer)?;
    if read == 16 && !answer.ends_with('\n') {
        return Ok(false); // Never accept a truncated prefix of a longer answer.
    }
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

fn download(asset: &ReleaseAsset, limit: u64, output: &mut dyn Write) -> Result<()> {
    ensure!(
        url::Url::parse(asset.download_url())?.scheme() == "https",
        "HTTPS の成果物 URL が必要です"
    );
    self_update::Download::from_url(asset.download_url())
        .request_header("Accept", "application/octet-stream")
        .timeout(Duration::from_secs(120))
        .max_download_size(limit)
        .download_to(output)
        .with_context(|| format!("成果物を取得できません: {}", asset.name()))?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    url: String,
    digest: Option<String>,
}

fn latest_release() -> Result<Release> {
    let mut metadata = Vec::new();
    self_update::Download::from_url(format!(
        "https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest"
    ))
    .request_header("Accept", "application/vnd.github+json")
    .request_header("User-Agent", "vss")
    .timeout(Duration::from_secs(60))
    .max_download_size(2 * 1024 * 1024)
    .download_to(&mut metadata)?;
    parse_release(&metadata)
}

fn parse_release(metadata: &[u8]) -> Result<Release> {
    let raw: GithubRelease = serde_json::from_slice(metadata)?;
    let version = canonical_version(&raw.tag_name)?;
    let assets: Vec<_> = raw
        .assets
        .into_iter()
        .map(|asset| {
            let item = ReleaseAsset::new(asset.name, asset.url);
            match asset.digest {
                Some(digest) => item.with_digest(digest),
                None => item,
            }
        })
        .collect();
    Ok(Release::builder()
        .version(version)
        .name(raw.tag_name)
        .date("")
        .assets(assets)
        .build()?)
}

fn exact_asset<'a>(release: &'a Release, name: &str) -> Result<&'a ReleaseAsset> {
    let mut assets = release.assets().iter().filter(|a| a.name() == name);
    let asset = assets
        .next()
        .with_context(|| format!("必須の成果物がありません: {name}"))?;
    ensure!(assets.next().is_none(), "成果物名が重複しています: {name}");
    Ok(asset)
}

// Strip exactly one optional lowercase v. Require canonical SemVer-compatible
// CalVer YYYY.M.MICRO; paths, padded segments and non-version tags cannot enter a filename.
fn canonical_version(tag: &str) -> Result<String> {
    let raw = tag.strip_prefix('v').unwrap_or(tag);
    let version = semver::Version::parse(raw)?;
    ensure!(
        (1000..=9999).contains(&version.major) && (1..=12).contains(&version.minor),
        "CalVer YYYY.M.MICRO が必要です: {tag}"
    );
    ensure!(version.to_string() == raw, "非正規のバージョンです: {tag}");
    Ok(raw.to_owned())
}

fn is_newer(current: &str, candidate: &str) -> Result<bool> {
    let order =
        semver::Version::parse(candidate)?.cmp_precedence(&semver::Version::parse(current)?);
    ensure!(
        !order.is_lt(),
        "ダウングレードは拒否されました: {current} → {candidate}"
    );
    Ok(order.is_gt())
}

fn asset_name(version: &str, target: &str) -> Result<String> {
    let version = canonical_version(version)?;
    let extension = match target {
        "linux_amd64" | "linux_arm64" | "darwin_arm64" => "tar.gz",
        "windows_amd64" => "zip",
        _ => bail!("Unsupported update target: {target}"),
    };
    Ok(format!("vss_{version}_{target}.{extension}"))
}

fn get_target() -> Result<&'static str> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

fn target_for(os: &str, arch: &str) -> Result<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("linux_amd64"),
        ("linux", "aarch64") => Ok("linux_arm64"),
        ("macos", "aarch64") => Ok("darwin_arm64"),
        ("windows", "x86_64") => Ok("windows_amd64"),
        _ => bail!("Unsupported platform: {os} {arch}. Install vss from source."),
    }
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_versions_sort_as_semver_for_self_update() {
        for (old, new) in [
            ("0.22.0", "2026.10.0"),
            ("2026.10.0", "2026.10.1"),
            ("2026.10.9", "2026.11.0"),
            ("2026.12.31", "2027.1.0"),
            ("2026.10.0-rc.1", "2026.10.0"),
        ] {
            assert!(self_update::version::bump_is_greater(old, new).unwrap());
            assert!(is_newer(old, new).unwrap());
        }
        assert!(!is_newer("2026.10.0", "2026.10.0").unwrap());
        assert!(!is_newer("2026.10.0+build.1", "2026.10.0+build.2").unwrap());
        assert!(is_newer("2026.10.1", "2026.10.0").is_err());
    }

    #[test]
    fn raw_github_tags_are_validated_before_backend_normalization() {
        for (tag, accepted) in [
            ("v2026.10.1", true),
            ("2026.10.1", true),
            ("vv2026.10.1", false),
            ("v2026.010.1", false),
            ("v2026.13.1", false),
            ("v2026.10.1/other", false),
        ] {
            let metadata = serde_json::json!({"tag_name": tag, "assets": []});
            assert_eq!(
                parse_release(&serde_json::to_vec(&metadata).unwrap()).is_ok(),
                accepted,
                "{tag}"
            );
        }
        let metadata = br#"{"tag_name":"v2026.10.1","assets":[{"name":"asset","url":"https://example.com","digest":"sha256:abc"}]}"#;
        let release = parse_release(metadata).unwrap();
        assert_eq!(release.version(), "2026.10.1");
        assert_eq!(release.assets()[0].digest(), Some("sha256:abc"));
    }

    #[test]
    fn canonical_names_and_exact_asset_selection() {
        let name = asset_name("v2026.10.1", "linux_amd64").unwrap();
        assert_eq!(name, "vss_2026.10.1_linux_amd64.tar.gz");
        assert_eq!(
            asset_name("2026.10.1", "windows_amd64").unwrap(),
            "vss_2026.10.1_windows_amd64.zip"
        );
        for tag in [
            "vv2026.10.0",
            "2026.010.0",
            "2026.13.0",
            "26.10.0",
            "../2026.10.0",
            "2026.10.0/evil",
            "2026.10.0$(id)",
            "0.22.0",
        ] {
            assert!(canonical_version(tag).is_err(), "{tag}");
        }
        let mut builder = Release::builder();
        builder.version("2026.10.1").name("release").date("");
        for wrong in [
            "vss_linux_amd64.tar.gz",
            "vss_2026.10.0_linux_amd64.tar.gz",
            "vss_2026.10.1_linux_arm64.tar.gz",
            "vss_2026.10.1_linux_amd64.tar.gz.sha256",
        ] {
            builder.asset(ReleaseAsset::new(wrong, "https://example.com"));
        }
        assert!(exact_asset(&builder.build().unwrap(), &name).is_err());
        builder.asset(ReleaseAsset::new(&name, "https://example.com"));
        assert_eq!(
            exact_asset(&builder.build().unwrap(), &name)
                .unwrap()
                .name(),
            name
        );
        builder.asset(ReleaseAsset::new(&name, "https://example.com"));
        assert!(exact_asset(&builder.build().unwrap(), &name).is_err());

        let archive_only = Release::builder()
            .version("2026.10.1")
            .name("release")
            .date("")
            .asset(ReleaseAsset::new(&name, "https://example.com"))
            .build()
            .unwrap();
        assert!(exact_asset(&archive_only, &format!("{name}.sha256")).is_err());
    }

    #[test]
    fn confirmation_defaults_to_no_and_yes_only_skips_prompt() {
        assert!(require_confirmation_mode(false, false).is_err());
        assert!(require_confirmation_mode(true, false).is_ok());
        assert!(require_confirmation_mode(false, true).is_ok());
        for (answer, expected) in [
            ("", false),
            ("\n", false),
            ("n\n", false),
            ("yes              not an approval\n", false),
            ("y\n", true),
            ("yes\n", true),
        ] {
            let mut output = Vec::new();
            assert_eq!(
                confirm(false, &mut answer.as_bytes(), &mut output).unwrap(),
                expected
            );
            assert!(!output.is_empty());
        }
        let mut output = Vec::new();
        assert!(confirm(true, &mut &b""[..], &mut output).unwrap());
        assert!(output.is_empty());
    }

    #[test]
    fn supported_platform_mappings_are_preserved() {
        for (os, arch, expected) in [
            ("linux", "x86_64", "linux_amd64"),
            ("linux", "aarch64", "linux_arm64"),
            ("macos", "aarch64", "darwin_arm64"),
            ("windows", "x86_64", "windows_amd64"),
        ] {
            assert_eq!(target_for(os, arch).unwrap(), expected);
        }
    }

    #[test]
    fn unsupported_platforms_do_not_panic() {
        for (os, arch) in [
            ("linux", "arm"),
            ("freebsd", "x86_64"),
            ("macos", "x86_64"),
            ("unknown", "unknown"),
        ] {
            let result = std::panic::catch_unwind(|| target_for(os, arch));
            assert!(result.is_ok(), "panicked for {os} {arch}");
            let message = result.unwrap().unwrap_err().to_string();
            assert!(message.contains(os) && message.contains(arch));
        }
    }
}
