use anyhow::{Context, Result};
use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
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
    // 末尾の / や /. で生成先リンクの検査を迂回させない。
    let destination: PathBuf = directory.components().collect();
    match fs::symlink_metadata(&destination) {
        Ok(metadata) => {
            anyhow::ensure!(
                !metadata.file_type().is_symlink(),
                "Destination is a symbolic link: {}",
                directory.display()
            );
            anyhow::ensure!(
                metadata.is_dir(),
                "Destination is not a directory: {}",
                directory.display()
            );
            anyhow::ensure!(
                fs::read_dir(directory)
                    .with_context(|| format!("Failed to read directory: {}", directory.display()))?
                    .next()
                    .transpose()?
                    .is_none(),
                "Destination directory is not empty: {}. Use an empty directory; existing files will not be overwritten.",
                directory.display()
            );
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            fs::create_dir_all(directory)
                .with_context(|| format!("Failed to create directory: {}", directory.display()))?;
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!("Failed to inspect destination: {}", directory.display())
            });
        }
    }

    for (name, contents) in SKELETON_FILES {
        let path = directory.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        // 事前検査後にファイルが作られた場合も、既存ファイルは上書きしない。
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("Failed to create file: {}", path.display()))?;
        // Windows でテンプレートが CRLF でチェックアウトされても、生成物は LF に統一する。
        file.write_all(contents.replace("\r\n", "\n").as_bytes())
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
    }
    Ok(())
}
