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

/// 変更されたファイルの種別
#[derive(Debug, Clone)]
pub enum ChangedFile {
    /// Markdown ファイルの変更
    Markdown(PathBuf),
    /// 静的ファイルの変更
    Static(PathBuf),
    /// テンプレートファイルの変更
    Template(PathBuf),
    /// 設定ファイルの変更
    Config,
    /// ファイル削除
    Deleted(PathBuf),
}

/// vss.toml の設定構造
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    #[serde(default = "default_site_title")]
    pub(crate) site_title: String,
    #[serde(default)]
    pub(crate) site_description: String,
    #[serde(default)]
    pub(crate) base_url: String,
    #[serde(default = "default_dist")]
    pub(crate) dist: String,
    #[serde(default = "default_static")]
    pub(crate) r#static: String,
    #[serde(default = "default_layouts")]
    pub(crate) layouts: String,
    #[serde(default)]
    pub(crate) build: BuildConfig,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct BuildConfig {
    #[serde(default)]
    pub(crate) ignore_files: Vec<String>,
    #[serde(default)]
    pub(crate) markdown: MarkdownConfig,
    #[serde(default)]
    pub(crate) tags: TagsConfig,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct MarkdownConfig {
    #[serde(default)]
    pub(crate) allow_dangerous_html: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TagsConfig {
    #[serde(default = "default_tags_enable")]
    pub(crate) enable: bool,
    #[serde(default = "default_tags_template")]
    pub(crate) template: String,
    #[serde(default = "default_tags_url_pattern")]
    pub(crate) url_pattern: String,
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
pub(crate) struct PostMetadata {
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
pub(crate) fn load_config(path: &Path) -> Result<Config> {
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
pub(crate) fn load_templates(layouts_dir: &str) -> Result<HashMap<String, ramhorns::Template<'static>>> {
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

    // 7. 投稿メタデータを収集
    let mut all_posts: Vec<PostMetadata> = Vec::new();

    // 8. 各 Markdown ファイルを処理
    for md_path in md_files {
        let post_metadata = process_markdown_file(&md_path, &config, &templates)?;
        // タグページ生成が有効な場合のみメタデータを収集
        if config.build.tags.enable
            && let Some(metadata) = post_metadata
        {
            all_posts.push(metadata);
        }
    }

    // 9. タグページを生成
    if !all_posts.is_empty() {
        generate_tag_pages(all_posts, &config, &templates)?;
    }

    Ok(())
}

/// 個別の Markdown ファイルを処理する
pub(crate) fn process_markdown_file(
    md_path: &Path,
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
) -> Result<Option<PostMetadata>> {
    // Markdown ファイルを読み込む
    let content = fs::read_to_string(md_path)
        .with_context(|| format!("Failed to read markdown file: {}", md_path.display()))?;

    // Frontmatter を解析
    let (frontmatter, markdown_content) = parse_frontmatter(&content)?;

    // Markdown を HTML に変換
    let html_content = markdown_to_html(
        &markdown_content,
        config.build.markdown.allow_dangerous_html,
    )?;

    // 出力パスを決定（.md → .html）
    let html_path = md_path.with_extension("html");
    let html_path_str = html_path.to_string_lossy().to_string();

    // テンプレートを検索
    let template = lookup_template(templates, &html_path_str)
        .context("No template found (default.html is required)")?;

    // タグ構造体を作成（url_pattern を使用）
    let tags: Vec<Tag> = frontmatter
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
        contents: html_content,
        title: frontmatter.title.clone(),
        description: frontmatter.description.clone(),
        author: frontmatter.author.clone(),
        pub_datetime: frontmatter.pub_datetime.clone(),
        post_slug: frontmatter.post_slug.clone(),
        has_tags,
        tags,
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

    // タグがある場合はメタデータを返す
    let metadata = frontmatter.tags.as_ref().and_then(|tags_vec| {
        if !tags_vec.is_empty() {
            let url = format!("/{}", html_path_str);
            Some(PostMetadata {
                title: frontmatter.title,
                description: frontmatter.description,
                author: frontmatter.author,
                pub_datetime: frontmatter.pub_datetime,
                url,
                tags: Some(tags_vec.clone()),
            })
        } else {
            None
        }
    });

    Ok(metadata)
}

/// タグページを生成する（設定を考慮）
fn generate_tag_pages(
    all_posts: Vec<PostMetadata>,
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
) -> Result<()> {
    // タグページ生成が無効な場合は何もしない
    if !config.build.tags.enable {
        println!("Tag page generation is disabled in config");
        return Ok(());
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

    // 各タグのページを生成
    for (tag_name, posts) in tag_to_posts {
        let context = TagPageContext {
            site_title: config.site_title.clone(),
            site_description: config.site_description.clone(),
            base_url: config.base_url.clone(),
            tag_name: tag_name.clone(),
            posts,
        };

        // テンプレート検索（設定から取得）
        let template = templates
            .get(&config.build.tags.template)
            .with_context(|| {
                format!(
                    "Tag template not found: layouts/{}",
                    config.build.tags.template
                )
            })?;

        // レンダリング
        let rendered = template.render(&context);

        // 出力先（設定のurl_patternから生成）
        // url_pattern: "/tags/{tag}/" -> output: "dist/tags/{tag}/index.html"
        let tag_path_str = config.build.tags.url_pattern.replace("{tag}", &tag_name);
        let relative_path = tag_path_str.trim_start_matches('/');

        // 指定された url_pattern が `/` で終わらない場合は `/tags/{tag}.html` のような path が
        // 指定されていると仮定して、`index.html` を path に結合しない
        let output_path = if relative_path.ends_with('/') {
            Path::new(&config.dist)
                .join(relative_path)
                .join("index.html")
        } else {
            Path::new(&config.dist).join(relative_path)
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(&output_path, rendered)
            .with_context(|| format!("Failed to write tag page: {}", output_path.display()))?;

        println!("Generated tag page: {}", output_path.display());
    }

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

/// 単一の静的ファイルをコピーする
pub(crate) fn copy_single_static_file(
    src_path: &Path,
    static_dir: &str,
    dist_dir: &str,
) -> Result<()> {
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
        fs::copy(src_path, &dest_path).with_context(|| {
            format!(
                "Failed to copy file from {} to {}",
                src_path.display(),
                dest_path.display()
            )
        })?;

        println!("Copied: {}", dest_path.display());
    }

    Ok(())
}

/// 出力ファイルを削除する（ソースファイル削除時に呼び出す）
pub(crate) fn delete_output_file(
    src_path: &Path,
    src_base_dir: &str,
    dist_dir: &str,
    convert_extension: Option<&str>,
) -> Result<()> {
    // ソースファイルからの相対パスを取得
    if let Ok(rel_path) = src_path.strip_prefix(src_base_dir) {
        let mut dest_path = Path::new(dist_dir).join(rel_path);

        // 拡張子変換が必要な場合（.md → .html）
        if let Some(ext) = convert_extension {
            dest_path.set_extension(ext);
        }

        // ファイルが存在する場合は削除
        if dest_path.exists() {
            fs::remove_file(&dest_path).with_context(|| {
                format!("Failed to delete output file: {}", dest_path.display())
            })?;
            println!("Deleted: {}", dest_path.display());
        }
    }

    Ok(())
}

/// テンプレートを使用する Markdown ファイルを検索
///
/// テンプレートの検索優先順位:
/// 1. 完全一致: layouts/{md_path}.html
/// 2. ディレクトリデフォルト: layouts/{dir}/default.html
/// 3. ルートデフォルト: layouts/default.html
///
/// テンプレートも一緒に返すことで、呼び出し側での再読み込みを避ける
pub(crate) fn find_markdown_files_using_template(
    template_path: &Path,
    layouts_dir: &str,
    ignore_files: &[String],
) -> Result<(Vec<PathBuf>, HashMap<String, ramhorns::Template<'static>>)> {
    let mut affected_files = Vec::new();

    // テンプレートを読み込む
    let templates = load_templates(layouts_dir)?;

    // テンプレートの相対パス（layouts/ からの相対）
    let template_rel_path = template_path
        .strip_prefix(layouts_dir)
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let Some(template_key) = template_rel_path else {
        return Ok((affected_files, templates));
    };

    // すべての Markdown ファイルを取得
    let md_files = find_files_with_glob("md").context("Failed to find markdown files")?;

    // ignore_files でフィルタリング
    let md_files: Vec<PathBuf> = md_files
        .into_iter()
        .filter(|path| {
            let path_str = path.to_string_lossy();
            !ignore_files.iter().any(|ignore| path_str.contains(ignore))
        })
        .collect();

    for md_path in md_files {
        let html_path = md_path.with_extension("html");
        let html_path_str = html_path.to_string_lossy().to_string();

        // このファイルが使用するテンプレートを特定
        let used_template_key = determine_template_key(&templates, &html_path_str);

        // 変更されたテンプレートを使用しているか確認
        if let Some(key) = used_template_key
            && key == template_key
        {
            affected_files.push(md_path);
        }
    }

    Ok((affected_files, templates))
}

/// Markdown ファイルが使用するテンプレートキーを決定する
fn determine_template_key(
    templates: &HashMap<String, ramhorns::Template<'static>>,
    html_path: &str,
) -> Option<String> {
    // 1. 完全一致
    if templates.contains_key(html_path) {
        return Some(html_path.to_string());
    }

    // 2. ディレクトリ内の default.html
    if let Some(dir) = Path::new(html_path).parent() {
        let dir_default = format!("{}/default.html", dir.display());
        if templates.contains_key(&dir_default) {
            return Some(dir_default);
        }
    }

    // 3. ルートの default.html
    if templates.contains_key("default.html") {
        return Some("default.html".to_string());
    }

    None
}

/// 絶対パスを相対パスに変換する
fn to_relative_path(path: &Path) -> PathBuf {
    if path.is_absolute()
        && let Ok(current_dir) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&current_dir)
    {
        return rel.to_path_buf();
    }
    path.to_path_buf()
}

/// 増分ビルドのエントリポイント
/// 変更されたファイルの種別に応じて最小限の再ビルドを行う
pub fn run_incremental_build(
    config_path: &Path,
    changed_files: &[ChangedFile],
) -> Result<()> {
    // 設定ファイルを読み込む
    let config = load_config(config_path)?;

    // テンプレートを読み込む
    let templates = load_templates(&config.layouts)?;

    // 投稿メタデータを収集（タグページ再生成用）
    let mut all_posts: Vec<PostMetadata> = Vec::new();
    let mut need_regenerate_tags = false;

    // カレントディレクトリを取得（パス正規化用）
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    for changed_file in changed_files {
        match changed_file {
            ChangedFile::Markdown(path) => {
                // 絶対パスを相対パスに変換
                let rel_path = to_relative_path(path);

                // Markdown ファイルが ignore_files に含まれているかチェック
                let path_str = rel_path.to_string_lossy();
                if config
                    .build
                    .ignore_files
                    .iter()
                    .any(|ignore| path_str.contains(ignore))
                {
                    continue;
                }

                // 単一の Markdown ファイルを処理（相対パスを使用）
                if let Ok(Some(metadata)) = process_markdown_file(&rel_path, &config, &templates) {
                    all_posts.push(metadata);
                }
                need_regenerate_tags = true;
            }
            ChangedFile::Static(path) => {
                // 単一の静的ファイルをコピー
                copy_single_static_file(path, &config.r#static, &config.dist)?;
            }
            ChangedFile::Template(path) => {
                // テンプレート変更時は、そのテンプレートを使用する全 Markdown を再ビルド
                // テンプレートも一緒に取得して再読み込みを避ける
                let (affected_md_files, fresh_templates) = find_markdown_files_using_template(
                    path,
                    &config.layouts,
                    &config.build.ignore_files,
                )?;

                for md_path in affected_md_files {
                    if let Ok(Some(metadata)) =
                        process_markdown_file(&md_path, &config, &fresh_templates)
                    {
                        all_posts.push(metadata);
                    }
                }
                need_regenerate_tags = true;
            }
            ChangedFile::Config => {
                // 設定ファイル変更時はフルビルド
                // この場合は run_build() を呼び出すべきなので、
                // 呼び出し側で処理する
                return Err(anyhow::anyhow!(
                    "Config changed, full rebuild required"
                ));
            }
            ChangedFile::Deleted(path) => {
                // ファイル削除時は対応する出力ファイルを削除
                let path_str = path.to_string_lossy();

                if path_str.ends_with(".md") {
                    // Markdown ファイルの削除
                    // 絶対パスを相対パスに変換してから処理
                    let rel_path = to_relative_path(path);
                    let html_path = rel_path.with_extension("html");
                    let output_path = Path::new(&config.dist).join(&html_path);
                    if output_path.exists() {
                        fs::remove_file(&output_path).with_context(|| {
                            format!("Failed to delete output file: {}", output_path.display())
                        })?;
                        println!("Deleted: {}", output_path.display());
                    }
                    need_regenerate_tags = true;
                } else if path.starts_with(current_dir.join(&config.r#static)) {
                    // 静的ファイルの削除
                    delete_output_file(path, &current_dir.join(&config.r#static).to_string_lossy(), &config.dist, None)?;
                } else if path.starts_with(current_dir.join(&config.layouts)) {
                    // テンプレートが削除された場合はフルビルドを要求
                    return Err(anyhow::anyhow!(
                        "Template file deleted, full rebuild required"
                    ));
                }
            }
        }
    }

    // タグページを再生成（Markdown の変更があった場合）
    if need_regenerate_tags && config.build.tags.enable {
        // 全 Markdown ファイルからメタデータを再収集してタグページを生成
        let md_files = find_files_with_glob("md").context("Failed to find markdown files")?;
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

        let mut tag_posts: Vec<PostMetadata> = Vec::new();
        for md_path in &md_files {
            // frontmatter を読み取ってメタデータを収集
            if let Ok(content) = fs::read_to_string(md_path)
                && let Ok((frontmatter, _)) = parse_frontmatter(&content)
                && let Some(tags_vec) = &frontmatter.tags
                && !tags_vec.is_empty()
            {
                let html_path = md_path.with_extension("html");
                let url = format!("/{}", html_path.to_string_lossy());
                tag_posts.push(PostMetadata {
                    title: frontmatter.title,
                    description: frontmatter.description,
                    author: frontmatter.author,
                    pub_datetime: frontmatter.pub_datetime,
                    url,
                    tags: Some(tags_vec.clone()),
                });
            }
        }

        if !tag_posts.is_empty() {
            generate_tag_pages(tag_posts, &config, &templates)?;
        }
    }

    Ok(())
}
