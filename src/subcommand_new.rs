use anyhow::{Context, Result};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use std::{
    io::{ErrorKind, Write},
    path::{Component, Path, PathBuf},
};

const SKELETON_FILES: &[(&str, &str)] = &[
    ("vss.toml", include_str!("skeleton/vss.toml")),
    ("index.md", include_str!("skeleton/index.md")),
    (
        "layouts/default.html",
        include_str!("skeleton/layouts/default.html"),
    ),
    (
        "layouts/tags/default.html",
        include_str!("skeleton/layouts/tags/default.html"),
    ),
    (
        "static/style.css",
        include_str!("skeleton/static/style.css"),
    ),
    (".gitignore", include_str!("skeleton/.gitignore")),
    ("README.md", include_str!("skeleton/README.md")),
];

/// サイトの雛形を生成するコマンド。
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let directory: PathBuf = noargs::arg("<DIR>")
        .doc("生成先。存在しないか空のディレクトリのみ指定できます。既存ファイルは上書きしません")
        .example("my-site")
        .take(&mut args)
        .then(|arg| arg.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    generate_site(&directory).map_err(|error| format!("{error:#}"))?;
    println!("Created site: {}", directory.display());
    println!("Run 'vss build' or 'vss serve' in that directory.");
    Ok(())
}

fn generate_site(directory: &Path) -> Result<()> {
    let destination = open_destination(directory)?;
    anyhow::ensure!(
        destination.entries()?.next().transpose()?.is_none(),
        "Destination directory is not empty: {}. Use an empty directory; existing files will not be overwritten.",
        directory.display()
    );
    write_skeleton(&destination)
}

/// 一要素ずつ nofollow で開き、以降の操作を保持したハンドルから行う。
/// 検査後に祖先のパスがリンクへ置換されても、そのリンクを辿らない。
fn open_child(parent: &Dir, name: &Path) -> Result<Dir> {
    match parent.symlink_metadata(name) {
        Ok(metadata) => {
            anyhow::ensure!(
                !metadata.file_type().is_symlink(),
                "Destination contains a symbolic link: {}",
                name.display()
            );
            anyhow::ensure!(
                metadata.is_dir(),
                "Destination is not a directory: {}",
                name.display()
            );
        }
        Err(error) if error.kind() == ErrorKind::NotFound => match parent.create_dir(name) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error).context("Failed to create destination directory"),
        },
        Err(error) => return Err(error).context("Failed to inspect destination"),
    }
    parent.open_dir_nofollow(name).with_context(|| {
        format!(
            "Failed to open destination without following symbolic links: {}",
            name.display()
        )
    })
}

fn open_destination(directory: &Path) -> Result<Dir> {
    let mut components = directory.components().peekable();
    let mut anchor = PathBuf::new();
    while let Some(component @ (Component::Prefix(_) | Component::RootDir)) =
        components.peek().copied()
    {
        anchor.push(component.as_os_str());
        components.next();
    }
    if anchor.as_os_str().is_empty() {
        anchor.push(".");
    }
    let mut destination = Dir::open_ambient_dir(&anchor, ambient_authority())?;
    for component in components {
        destination = match component {
            Component::CurDir => destination,
            Component::ParentDir => destination.open_parent_dir(ambient_authority())?,
            Component::Normal(name) => open_child(&destination, Path::new(name))?,
            _ => anyhow::bail!("Invalid destination path"),
        };
    }
    Ok(destination)
}

fn write_skeleton(destination: &Dir) -> Result<()> {
    for (name, contents) in SKELETON_FILES {
        let path = Path::new(name);
        let mut parent = destination.try_clone()?;
        if let Some(directories) = path.parent() {
            for name in directories.iter() {
                parent = open_child(&parent, Path::new(name))?;
            }
        }
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let mut file = parent
            .open_with(Path::new(path.file_name().unwrap()), &options)
            .with_context(|| format!("Failed to create file: {name}"))?;
        // Windows の CRLF チェックアウトでも生成物は LF に統一する。
        file.write_all(contents.replace("\r\n", "\n").as_bytes())?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn replacing_an_opened_ancestor_with_a_symlink_cannot_redirect_writes() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        fs::create_dir(root.join("parent")).unwrap();
        fs::create_dir(root.join("victim")).unwrap();
        let destination = open_destination(&root.join("parent/site")).unwrap();
        fs::rename(root.join("parent"), root.join("moved")).unwrap();
        symlink(root.join("victim"), root.join("parent")).unwrap();
        write_skeleton(&destination).unwrap();
        assert_eq!(fs::read_dir(root.join("victim")).unwrap().count(), 0);
        assert!(root.join("moved/site/vss.toml").is_file());
    }

    #[test]
    fn rejects_a_skeleton_directory_replaced_with_a_symlink() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        fs::create_dir(root.join("site")).unwrap();
        fs::create_dir(root.join("victim")).unwrap();
        let destination = open_destination(&root.join("site")).unwrap();
        symlink(root.join("victim"), root.join("site/layouts")).unwrap();
        assert!(write_skeleton(&destination).is_err());
        assert_eq!(fs::read_dir(root.join("victim")).unwrap().count(), 0);
    }
}
