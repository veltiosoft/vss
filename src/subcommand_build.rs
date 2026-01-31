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

// ============================================================================
// Pipeline Data Structures
// ============================================================================

/// パース済み Markdown ドキュメント
struct ParsedDocument {
    source_path: PathBuf,
    frontmatter: FrontMatter,
    html_content: String,
}

/// レンダリング済みページ
struct RenderedPage {
    output_path: PathBuf,
    html: String,
    metadata: Option<PostMetadata>,
}

/// 出力ファイル
struct OutputFile {
    path: PathBuf,
    content: String,
}

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
    #[serde(default)]
    markdown: MarkdownConfig,
    #[serde(default)]
    tags: TagsConfig,
}

#[derive(Debug, Deserialize, Default)]
struct MarkdownConfig {
    #[serde(default)]
    allow_dangerous_html: bool,
}

#[derive(Debug, Deserialize)]
struct TagsConfig {
    #[serde(default = "default_tags_enable")]
    enable: bool,
    #[serde(default = "default_tags_template")]
    template: String,
    #[serde(default = "default_tags_url_pattern")]
    url_pattern: String,
}

impl Default for TagsConfig {
    fn default() -> Self {
        Self {
            enable: default_tags_enable(),
            template: default_tags_template(),
            url_pattern: default_tags_url_pattern(),
        }
    }
}

fn default_tags_enable() -> bool {
    true
}

fn default_tags_template() -> String {
    "tags/default.html".to_string()
}

fn default_tags_url_pattern() -> String {
    "/tags/{tag}/".to_string()
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
    tags: Option<Vec<String>>,
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
    has_tags: bool,
    tags: Vec<Tag>,
}

/// タグ表示用の構造体
#[derive(Content)]
struct Tag {
    name: String,
    url: String,
}

/// タグページ生成用の投稿メタデータ
#[derive(Content, Clone)]
struct PostMetadata {
    title: String,
    description: String,
    author: String,
    pub_datetime: String,
    url: String,
    tags: Option<Vec<String>>,
}

/// タグページのレンダリングコンテキスト
#[derive(Content)]
struct TagPageContext {
    site_title: String,
    site_description: String,
    base_url: String,
    tag_name: String,
    posts: Vec<PostMetadata>,
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
fn markdown_to_html(markdown: &str, allow_dangerous_html: bool) -> Result<String> {
    let mut options = markdown::Options::gfm();
    if allow_dangerous_html {
        options.compile.allow_dangerous_html = true;
    }

    markdown::to_html_with_options(markdown, &options)
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

// ============================================================================
// Phase 1: Pure Functions (Collection & Transformation)
// ============================================================================

/// Markdown ファイルを収集してコンテンツを読み込む
fn collect_markdown_sources(config: &Config) -> Result<Vec<(PathBuf, String)>> {
    let md_files = find_files_with_glob("md").context("Failed to find markdown files")?;

    // ignore_files でフィルタリング
    let filtered: Vec<PathBuf> = md_files
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

    // ファイルコンテンツを読み込む
    filtered
        .into_iter()
        .map(|path| {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read markdown file: {}", path.display()))?;
            Ok((path, content))
        })
        .collect()
}

/// 単一ドキュメントをパース
fn parse_document(source_path: PathBuf, content: &str, config: &Config) -> Result<ParsedDocument> {
    let (frontmatter, markdown_content) = parse_frontmatter(content)?;
    let html_content = markdown_to_html(
        &markdown_content,
        config.build.markdown.allow_dangerous_html,
    )?;

    Ok(ParsedDocument {
        source_path,
        frontmatter,
        html_content,
    })
}

/// 単一ページをレンダリング
fn render_page(
    doc: ParsedDocument,
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
) -> Result<RenderedPage> {
    // 出力パスを決定（.md → .html）
    let html_path = doc.source_path.with_extension("html");
    let html_path_str = html_path.to_string_lossy().to_string();

    // テンプレートを検索
    let template = lookup_template(templates, &html_path_str)
        .context("No template found (default.html is required)")?;

    // タグ構造体を作成
    let tags: Vec<Tag> = doc
        .frontmatter
        .tags
        .as_ref()
        .map(|tags_vec| {
            tags_vec
                .iter()
                .map(|tag_name| Tag {
                    name: tag_name.clone(),
                    url: config.build.tags.url_pattern.replace("{tag}", tag_name),
                })
                .collect()
        })
        .unwrap_or_default();

    // レンダリングコンテキストを構築
    let has_tags = !tags.is_empty();
    let context = RenderContext {
        site_title: config.site_title.clone(),
        site_description: config.site_description.clone(),
        base_url: config.base_url.clone(),
        contents: doc.html_content,
        title: doc.frontmatter.title.clone(),
        description: doc.frontmatter.description.clone(),
        author: doc.frontmatter.author.clone(),
        pub_datetime: doc.frontmatter.pub_datetime.clone(),
        post_slug: doc.frontmatter.post_slug.clone(),
        has_tags,
        tags,
    };

    // テンプレートをレンダリング
    let html = template.render(&context);

    // 出力先を決定
    let output_path = Path::new(&config.dist).join(&html_path);

    println!("Generated: {}", output_path.display());

    // メタデータを生成（タグがある場合のみ）
    let metadata = if config.build.tags.enable {
        doc.frontmatter.tags.as_ref().and_then(|tags_vec| {
            if !tags_vec.is_empty() {
                let url = format!("/{}", html_path_str);
                Some(PostMetadata {
                    title: doc.frontmatter.title,
                    description: doc.frontmatter.description,
                    author: doc.frontmatter.author,
                    pub_datetime: doc.frontmatter.pub_datetime,
                    url,
                    tags: Some(tags_vec.clone()),
                })
            } else {
                None
            }
        })
    } else {
        None
    };

    Ok(RenderedPage {
        output_path,
        html,
        metadata,
    })
}

/// タグページの OutputFile を生成
fn generate_tag_outputs(
    rendered_pages: &[RenderedPage],
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
) -> Result<Vec<OutputFile>> {
    // タグページ生成が無効な場合は空を返す
    if !config.build.tags.enable {
        return Ok(Vec::new());
    }

    // メタデータを収集
    let all_posts: Vec<PostMetadata> = rendered_pages
        .iter()
        .filter_map(|p| p.metadata.clone())
        .collect();

    if all_posts.is_empty() {
        return Ok(Vec::new());
    }

    // タグごとに投稿をグループ化
    let mut tag_to_posts: HashMap<String, Vec<PostMetadata>> = HashMap::new();

    for post in all_posts {
        if let Some(tags_vec) = &post.tags {
            for tag_name in tags_vec {
                tag_to_posts
                    .entry(tag_name.clone())
                    .or_default()
                    .push(post.clone());
            }
        }
    }

    // テンプレートを取得
    let template = templates
        .get(&config.build.tags.template)
        .with_context(|| {
            format!(
                "Tag template not found: layouts/{}",
                config.build.tags.template
            )
        })?;

    // 各タグの OutputFile を生成
    let outputs: Vec<OutputFile> = tag_to_posts
        .into_iter()
        .map(|(tag_name, posts)| {
            let context = TagPageContext {
                site_title: config.site_title.clone(),
                site_description: config.site_description.clone(),
                base_url: config.base_url.clone(),
                tag_name: tag_name.clone(),
                posts,
            };

            let content = template.render(&context);

            // 出力先を決定
            let tag_path_str = config.build.tags.url_pattern.replace("{tag}", &tag_name);
            let relative_path = tag_path_str.trim_start_matches('/');

            let path = if relative_path.ends_with('/') {
                Path::new(&config.dist)
                    .join(relative_path)
                    .join("index.html")
            } else {
                Path::new(&config.dist).join(relative_path)
            };

            println!("Generated tag page: {}", path.display());

            OutputFile { path, content }
        })
        .collect();

    Ok(outputs)
}

// ============================================================================
// Phase 2: Side Effects (Output)
// ============================================================================

/// すべての出力ファイルを書き出す
fn write_all_outputs(outputs: impl Iterator<Item = OutputFile>) -> Result<()> {
    for output in outputs {
        // 親ディレクトリを作成
        if let Some(parent) = output.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // ファイルに書き込む
        fs::write(&output.path, &output.content)
            .with_context(|| format!("Failed to write output file: {}", output.path.display()))?;
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
    // 設定ファイルを読み込む
    let config = load_config(config_path)?;

    // テンプレートを読み込む
    let templates = load_templates(&config.layouts)?;

    // ========================================================================
    // Phase 1: 収集 & 変換（純粋関数）
    // ========================================================================

    // Markdown ソースを収集
    let sources = collect_markdown_sources(&config)?;

    // ドキュメントをパース
    let parsed_docs: Vec<ParsedDocument> = sources
        .into_iter()
        .map(|(path, content)| parse_document(path, &content, &config))
        .collect::<Result<Vec<_>>>()?;

    // ページをレンダリング
    let rendered_pages: Vec<RenderedPage> = parsed_docs
        .into_iter()
        .map(|doc| render_page(doc, &config, &templates))
        .collect::<Result<Vec<_>>>()?;

    // タグページの出力を生成
    let tag_outputs = generate_tag_outputs(&rendered_pages, &config, &templates)?;

    // ========================================================================
    // Phase 2: 出力（副作用）
    // ========================================================================

    // dist ディレクトリを作成
    create_dist_dir(&config.dist)?;

    // 静的ファイルをコピー
    copy_static_files(&config.r#static, &config.dist)?;

    // レンダリング済みページを書き出し
    write_all_outputs(
        rendered_pages.iter().map(|p| OutputFile {
            path: p.output_path.clone(),
            content: p.html.clone(),
        }),
    )?;

    // タグページを書き出し
    write_all_outputs(tag_outputs.into_iter())?;

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
