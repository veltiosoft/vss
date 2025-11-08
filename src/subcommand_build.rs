use anyhow::{Context, Result};
use glob::glob;
use ramhorns::Content;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

/// vss.toml の設定構造
#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "default_site_title")]
    site_title: String,
    #[serde(default)]
    site_description: String,
    #[serde(default)]
    base_url: String,
    #[serde(default = "default_dist")]
    dist: String,
    #[serde(default = "default_static")]
    r#static: String,
    #[serde(default = "default_layouts")]
    layouts: String,
    #[serde(default)]
    build: BuildConfig,
}

#[derive(Debug, Deserialize, Default)]
struct BuildConfig {
    #[serde(default)]
    ignore_files: Vec<String>,
}

fn default_site_title() -> String {
    "vss site".to_string()
}

fn default_dist() -> String {
    "dist".to_string()
}

fn default_static() -> String {
    "static".to_string()
}

fn default_layouts() -> String {
    "layouts".to_string()
}

/// YAML frontmatter の構造
#[derive(Debug, Deserialize, Default)]
struct FrontMatter {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    pub_datetime: String,
    #[serde(default)]
    post_slug: String,
    #[serde(default)]
    #[allow(dead_code)]
    tags: Vec<String>,
}

/// テンプレートレンダリング用のコンテキスト
#[derive(Content)]
struct RenderContext {
    site_title: String,
    site_description: String,
    base_url: String,
    contents: String,
    title: String,
    description: String,
    author: String,
    pub_datetime: String,
    post_slug: String,
}

/// 設定ファイルを読み込む
fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Frontmatter とコンテンツを解析する
fn parse_frontmatter(content: &str) -> Result<(FrontMatter, String)> {
    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let parsed = matter.parse(content);

    let frontmatter = if let Some(data) = parsed.data {
        // Pod を deserialize する
        data.deserialize().unwrap_or_default()
    } else {
        FrontMatter::default()
    };

    Ok((frontmatter, parsed.content))
}

/// Markdown を HTML に変換する
fn markdown_to_html(markdown: &str) -> Result<String> {
    markdown::to_html_with_options(markdown, &markdown::Options::gfm())
        .map_err(|e| anyhow::anyhow!("Failed to convert markdown to HTML: {}", e))
}

/// テンプレートを読み込んでキャッシュする
fn load_templates(layouts_dir: &str) -> Result<HashMap<String, ramhorns::Template<'static>>> {
    let mut templates = HashMap::new();

    let pattern = format!("{}/**/*.html", layouts_dir);
    for entry in glob(&pattern).context("Failed to read template glob pattern")? {
        let path = entry.context("Failed to read template entry")?;
        if path.is_file() {
            let template_content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read template: {}", path.display()))?;
            let template = ramhorns::Template::new(template_content)
                .with_context(|| format!("Failed to parse template: {}", path.display()))?;

            // layouts/ からの相対パスをキーとする
            if let Ok(rel_path) = path.strip_prefix(layouts_dir) {
                let key = rel_path.to_string_lossy().to_string();
                templates.insert(key, template);
            }
        }
    }

    Ok(templates)
}

/// テンプレートを検索する（3段階の優先順位）
fn lookup_template<'a>(
    templates: &'a HashMap<String, ramhorns::Template<'a>>,
    html_path: &str,
) -> Option<&'a ramhorns::Template<'a>> {
    // 1. 完全一致
    if let Some(template) = templates.get(html_path) {
        return Some(template);
    }

    // 2. ディレクトリ内の default.html
    if let Some(dir) = Path::new(html_path).parent() {
        let dir_default = format!("{}/default.html", dir.display());
        if let Some(template) = templates.get(&dir_default) {
            return Some(template);
        }
    }

    // 3. ルートの default.html
    templates.get("default.html")
}

/// dist ディレクトリを作成する（既存の場合は削除して再作成）
fn create_dist_dir(dist_path: &str) -> Result<()> {
    let path = Path::new(dist_path);
    if path.exists() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove existing dist directory: {}", dist_path))?;
    }
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create dist directory: {}", dist_path))?;
    Ok(())
}

/// 静的ファイルを再帰的にコピーする
fn copy_static_files(static_dir: &str, dist_dir: &str) -> Result<()> {
    let static_path = Path::new(static_dir);
    if !static_path.exists() {
        // static ディレクトリがなければスキップ
        return Ok(());
    }

    let pattern = format!("{}/**/*", static_dir);
    for entry in glob(&pattern).context("Failed to read static files glob pattern")? {
        let src_path = entry.context("Failed to read static file entry")?;
        if src_path.is_file() {
            // static/ からの相対パスを取得
            if let Ok(rel_path) = src_path.strip_prefix(static_dir) {
                let dest_path = Path::new(dist_dir).join(rel_path);

                // 親ディレクトリを作成
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create directory: {}", parent.display())
                    })?;
                }

                // ファイルをコピー
                fs::copy(&src_path, &dest_path).with_context(|| {
                    format!(
                        "Failed to copy file from {} to {}",
                        src_path.display(),
                        dest_path.display()
                    )
                })?;
            }
        }
    }

    Ok(())
}

/// ビルドコマンドのエントリポイント。
pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let config: Option<PathBuf> = noargs::opt("config")
        .ty("PATH")
        .example("/path/to/vss.toml")
        .doc("設定ファイルパス")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    // config が指定されていない場合はデフォルトで
    // 現在のディレクトリの vss.toml を利用する。
    let config_path = match config {
        Some(p) => p,
        None => PathBuf::from("vss.toml"),
    };

    // 処理時間を計測する
    let start = Instant::now();

    // ビルド実行
    if let Err(e) = run_build(&config_path) {
        eprintln!("Build failed: {:#}", e);
        std::process::exit(1);
    }

    let duration = start.elapsed();
    println!("build finished in {} ms", duration.as_millis());
    Ok(())
}

/// 実際のビルド処理
pub fn run_build(config_path: &Path) -> Result<()> {
    // 1. 設定ファイルを読み込む
    let config = load_config(config_path)?;

    // 2. dist ディレクトリを作成
    create_dist_dir(&config.dist)?;

    // 3. 静的ファイルをコピー
    copy_static_files(&config.r#static, &config.dist)?;

    // 4. Markdown ファイルを検索
    let md_files = find_files_with_glob("md").context("Failed to find markdown files")?;

    // 5. ignore_files でフィルタリング
    let md_files: Vec<PathBuf> = md_files
        .into_iter()
        .filter(|path| {
            let path_str = path.to_string_lossy();
            !config
                .build
                .ignore_files
                .iter()
                .any(|ignore| path_str.contains(ignore))
        })
        .collect();

    // 6. テンプレートを読み込む
    let templates = load_templates(&config.layouts)?;

    // 7. 各 Markdown ファイルを処理
    for md_path in md_files {
        process_markdown_file(&md_path, &config, &templates)?;
    }

    Ok(())
}

/// 個別の Markdown ファイルを処理する
fn process_markdown_file(
    md_path: &Path,
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
) -> Result<()> {
    // Markdown ファイルを読み込む
    let content = fs::read_to_string(md_path)
        .with_context(|| format!("Failed to read markdown file: {}", md_path.display()))?;

    // Frontmatter を解析
    let (frontmatter, markdown_content) = parse_frontmatter(&content)?;

    // Markdown を HTML に変換
    let html_content = markdown_to_html(&markdown_content)?;

    // 出力パスを決定（.md → .html）
    let html_path = md_path.with_extension("html");
    let html_path_str = html_path.to_string_lossy().to_string();

    // テンプレートを検索
    let template = lookup_template(templates, &html_path_str)
        .context("No template found (default.html is required)")?;

    // レンダリングコンテキストを構築
    let context = RenderContext {
        site_title: config.site_title.clone(),
        site_description: config.site_description.clone(),
        base_url: config.base_url.clone(),
        contents: html_content,
        title: frontmatter.title,
        description: frontmatter.description,
        author: frontmatter.author,
        pub_datetime: frontmatter.pub_datetime,
        post_slug: frontmatter.post_slug,
    };

    // テンプレートをレンダリング
    let rendered = template.render(&context);

    // 出力先を決定
    let output_path = Path::new(&config.dist).join(&html_path);

    // 親ディレクトリを作成
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // ファイルに書き込む
    fs::write(&output_path, rendered)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!("Generated: {}", output_path.display());

    Ok(())
}

fn find_files_with_glob(extension: &str) -> Result<Vec<PathBuf>, glob::PatternError> {
    // ** は任意の深さのディレクトリ、* は任意のファイル名にマッチ
    let pattern = format!("**/*.{}", extension);
    let mut files = Vec::new();

    for entry in glob(&pattern)? {
        match entry {
            Ok(path) => {
                // ファイルであることを確認する処理を追加することもできますが、
                // 通常は glob パターンがファイルのみにマッチすると期待されます。
                if path.is_file() {
                    files.push(path);
                }
            }
            Err(e) => eprintln!("Error processing glob entry: {:?}", e),
        }
    }
    Ok(files)
}
