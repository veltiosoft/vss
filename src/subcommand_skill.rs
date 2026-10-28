use anyhow::{Context, Result};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

const INSTALL_COMMAND: noargs::CmdSpec =
    noargs::cmd("install").doc("同梱 SKILL を配置します (例: vss skill install --dir ./skills)");
const SKILL: &str = include_str!("../skills/vss/SKILL.md");

/// Agent 固有の保存先は選択せず、明示された親ディレクトリへ配置する。
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    if INSTALL_COMMAND.take(&mut args).is_present() {
        run_install(args)
    } else if let Some(help) = args.finish()? {
        print!("{help}");
        Ok(())
    } else {
        Err("使用方法: vss skill install --dir <DIR>".into())
    }
}

fn run_install(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let directory: PathBuf = noargs::opt("dir")
        .ty("DIR")
        .doc("必須。SKILL の親ディレクトリ。未作成なら作成し、配下の vss/SKILL.md に配置します。既存の vss は上書きしません")
        .example("./skills")
        .take(&mut args)
        .then(|opt| {
            if opt.value().is_empty() {
                Err("SKILL の親ディレクトリに空のパスは指定できません")
            } else {
                Ok(PathBuf::from(opt.value()))
            }
        })?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let installed = install(&directory).map_err(|error| format!("{error:#}"))?;
    println!("Installed skill: {}", installed.display());
    Ok(())
}

fn install(directory: &Path) -> Result<PathBuf> {
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
    // create_dir は空のディレクトリや壊れたリンクも含めて既存パスを拒否する。
    parent.create_dir("vss").map_err(|error| {
        if error.kind() == ErrorKind::AlreadyExists {
            anyhow::anyhow!(
                "Skill destination already exists: {}. Existing files will not be overwritten.",
                destination.display()
            )
        } else {
            anyhow::Error::new(error).context(format!(
                "Failed to create skill destination: {}",
                destination.display()
            ))
        }
    })?;
    // 作成後のリンク置換を辿らず、保持したディレクトリハンドルから書き込む。
    let skill_dir = parent.open_dir_nofollow("vss").with_context(|| {
        format!(
            "Failed to open skill destination: {}",
            destination.display()
        )
    })?;
    let installed = destination.join("SKILL.md");
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut file = skill_dir
        .open_with("SKILL.md", &options)
        .with_context(|| format!("Failed to create skill file: {}", installed.display()))?;
    file.write_all(SKILL.replace("\r\n", "\n").as_bytes())
        .with_context(|| format!("Failed to write skill file: {}", installed.display()))?;
    Ok(installed)
}
