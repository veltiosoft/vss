use anyhow::{Context, Result};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use cap_tempfile::TempFile;
use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

const INSTALL_COMMAND: noargs::CmdSpec =
    noargs::cmd("install").doc("指定した親ディレクトリに同梱スキルを配置します");
const SKILL_FILE: &str = "SKILL.md";
const SKILL: &str = include_str!("../skills/vss/SKILL.md");

/// Agent 固有の保存先は選択せず、明示された親ディレクトリへ配置する。
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    if INSTALL_COMMAND.take(&mut args).is_present() {
        run_install(args)
    } else {
        crate::cli::finish(args, "skill")?;
        Ok(())
    }
}

fn run_install(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let force = noargs::flag("force")
        .doc("既存の vss/SKILL.md を同梱版に置き換えます。他のファイルは変更しません")
        .take(&mut args)
        .is_present();
    let directory: PathBuf = noargs::opt("dir")
        .ty("DIR")
        .doc("必須。SKILL の親ディレクトリ。未作成なら作成し、配下の vss/SKILL.md に配置します")
        .example("./skills")
        .take(&mut args)
        .then(|opt| {
            if opt.value().is_empty() {
                Err("SKILL の親ディレクトリに空のパスは指定できません")
            } else {
                Ok(PathBuf::from(opt.value()))
            }
        })?;
    if crate::cli::finish(args, "skill install")? {
        return Ok(());
    }

    let installed = install(&directory, force).map_err(|error| format!("{error:#}"))?;
    println!("Installed skill: {}", installed.display());
    Ok(())
}

fn install(directory: &Path, force: bool) -> Result<PathBuf> {
    fs::create_dir_all(directory).with_context(|| {
        format!(
            "Failed to create skill parent directory: {}",
            directory.display()
        )
    })?;
    let directory = directory.canonicalize().with_context(|| {
        format!(
            "Failed to resolve skill parent directory: {}",
            directory.display()
        )
    })?;
    let parent = Dir::open_ambient_dir(&directory, ambient_authority()).with_context(|| {
        format!(
            "Failed to open skill parent directory: {}",
            directory.display()
        )
    })?;
    let destination = directory.join("vss");
    // 既存の vss は --force 指定時に限り、後続の nofollow 検査で確認して再利用する。
    match parent.create_dir("vss") {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => anyhow::ensure!(
            force,
            "Skill destination already exists: {}. Use --force to replace only vss/SKILL.md in an existing directory.",
            destination.display()
        ),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "Failed to create skill destination: {}",
                    destination.display()
                )
            });
        }
    }
    // 作成後のリンク置換を辿らず、保持したディレクトリハンドルから書き込む。
    let skill_dir = parent.open_dir_nofollow("vss").with_context(|| {
        format!(
            "Failed to open skill destination: {}",
            destination.display()
        )
    })?;
    let installed = destination.join(SKILL_FILE);
    let contents = SKILL.replace("\r\n", "\n");
    if force {
        replace_skill(&skill_dir, contents.as_bytes())
            .with_context(|| format!("Failed to replace skill file: {}", installed.display()))?;
    } else {
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let mut file = skill_dir
            .open_with(SKILL_FILE, &options)
            .with_context(|| format!("Failed to create skill file: {}", installed.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("Failed to write skill file: {}", installed.display()))?;
    }
    Ok(installed)
}

fn replace_skill(skill_dir: &Dir, contents: &[u8]) -> Result<()> {
    // 既存の内容は切り詰めず、新しいファイルを書き切ってから名前を置き換える。
    let mut staged = TempFile::new(skill_dir)?;
    staged.write_all(contents)?;
    match skill_dir.symlink_metadata(SKILL_FILE) {
        Ok(metadata) => anyhow::ensure!(metadata.is_file(), "SKILL.md is not a regular file"),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    staged.replace(SKILL_FILE)?;
    Ok(())
}
