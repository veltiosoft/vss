use anyhow::{Context, Result};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use ramhorns::Content;
use serde::{
    Deserialize, Deserializer,
    de::{self, SeqAccess, Visitor},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    time::Instant,
};
use walkdir::WalkDir;

/// 変更されたファイルの種別
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ChangedFile {
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

const PARTIALS_DIR: &str = "partials";
const MAX_FRONTMATTER_BYTES: usize = 65_536;
const MAX_TAGS: usize = 128;
const MAX_TAG_BYTES: usize = 256;
const URL_SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

#[cfg(test)]
mod tests;

/// vss.toml の設定構造
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    #[serde(default = "default_site_title")]
    site_title: String,
    #[serde(default)]
    site_description: String,
    #[serde(default)]
    base_url: String,
    #[serde(default = "default_dist")]
    pub(crate) dist: String,
    #[serde(default = "default_static")]
    pub(crate) r#static: String,
    #[serde(default = "default_layouts")]
    pub(crate) layouts: String,
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

/// タグ名が安全かどうかを検証する
/// パストラバーサル攻撃を防ぐため、危険な文字列を含むタグ名を拒否する
fn is_safe_tag_name(tag: &str) -> bool {
    // 空のタグ名は不正
    if tag.is_empty() || tag == "." {
        return false;
    }

    // パストラバーサルシーケンスを含む場合は不正
    if tag.contains("..") || tag.contains('/') || tag.contains('\\') {
        return false;
    }

    // URL スキーム、Windows のドライブ接頭辞、制御文字を受け付けない。
    if tag.contains(':') || tag.chars().any(char::is_control) {
        return false;
    }

    true
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
    og_image: String,
    #[serde(default, deserialize_with = "deserialize_tags")]
    tags: Option<Vec<String>>,
}

fn deserialize_tags<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error> {
    struct TagsVisitor;

    impl<'de> Visitor<'de> for TagsVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "at most {MAX_TAGS} tags of at most {MAX_TAG_BYTES} bytes each"
            )
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_seq(self)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut tags = Vec::new();
            while let Some(tag) = sequence.next_element::<String>()? {
                if tags.len() == MAX_TAGS {
                    return Err(de::Error::custom(format!(
                        "Too many tags (maximum {MAX_TAGS})"
                    )));
                }
                if tag.len() > MAX_TAG_BYTES {
                    return Err(de::Error::custom(format!(
                        "Tag exceeds {MAX_TAG_BYTES} bytes"
                    )));
                }
                tags.push(tag);
            }
            Ok(Some(tags))
        }
    }

    deserializer.deserialize_option(TagsVisitor)
}

/// テンプレートレンダリング用のコンテキスト
#[derive(Content)]
struct RenderContext<'a> {
    site_title: String,
    site_description: String,
    base_url: String,
    contents: String,
    title: String,
    description: String,
    author: String,
    pub_datetime: String,
    post_slug: String,
    og_image: String,
    og_image_url: String,
    has_tags: bool,
    tags: Vec<Tag>,
    // ramhorns はドットを含む名前をそのまま参照するため、partials.* を展開する。
    #[ramhorns(flatten)]
    partials: &'a HashMap<String, String>,
}

/// タグ表示用の構造体
#[derive(Content)]
struct Tag {
    name: String,
    url: String,
}

/// タグページ生成用の投稿メタデータ
#[derive(Content)]
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
struct TagPageContext<'a> {
    site_title: String,
    site_description: String,
    base_url: String,
    tag_name: String,
    posts: Vec<&'a PostMetadata>,
    #[ramhorns(flatten)]
    partials: &'a HashMap<String, String>,
}

/// 設定ファイルを読み込む
pub(crate) fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    let pattern = &config.build.tags.url_pattern;
    let relative = pattern.trim_start_matches('/');
    anyhow::ensure!(
        !relative.is_empty()
            && !pattern.starts_with("//")
            && !pattern.contains([':', '\\'])
            && !pattern.chars().any(char::is_control)
            && !relative.split('/').any(|part| matches!(part, "." | "..")),
        "Tag URL pattern must be a path inside the site"
    );
    let root = std::env::current_dir().context("Failed to get current directory")?;
    for (setting, value) in [("static", &config.r#static), ("layouts", &config.layouts)] {
        let directory = site_directory(&root, value, setting)?;
        anyhow::ensure!(
            !has_private_source_component(&root, &directory)?,
            "{setting} must not refer to private directories"
        );
    }
    validate_dist(&config, path)?;
    Ok(config)
}

/// 設定パスは OS にかかわらずサイト内の相対パスだけを受け付ける。
fn site_directory(root: &Path, value: &str, setting: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        !value.is_empty()
            && !Path::new(value).is_absolute()
            && !value.contains([':', '\\'])
            && !value.chars().any(char::is_control)
            && !value.split('/').any(|part| part == ".."),
        "{setting} must be a relative directory inside the site"
    );
    let directory = absolute_path(root, Path::new(value));
    anyhow::ensure!(
        directory != root && directory.starts_with(root),
        "{setting} must not be the site root or outside the site"
    );
    Ok(directory)
}

fn has_private_source_component(root: &Path, directory: &Path) -> Result<bool> {
    for ancestor in directory.ancestors().take_while(|path| *path != root) {
        let name = ancestor
            .file_name()
            .context("Invalid source directory")?
            .to_string_lossy();
        // Win32 は末尾のドット・空白を取り除いて解決する。
        let name = name.trim_end_matches(['.', ' ']);
        for private in [".git", ".ssh", ".env"] {
            if name.eq_ignore_ascii_case(private) {
                return Ok(true);
            }
            // Windows の短縮名など、文字列だけでは判別できない別名も拒否する。
            let protected = ancestor
                .parent()
                .context("Invalid source parent")?
                .join(private);
            match same_file::is_same_file(ancestor, protected) {
                Ok(true) => return Ok(true),
                Ok(false) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).context("Failed to inspect private source aliases");
                }
            }
        }
    }
    Ok(false)
}

fn ensure_no_symlink_ancestors(path: &Path, setting: &str) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => anyhow::ensure!(
                !metadata.file_type().is_symlink(),
                "{setting} must not contain symbolic links: {}",
                ancestor.display()
            ),
            // 未作成の出力でも、既存の祖先はすべて検査する。
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Failed to inspect {setting}: {}", ancestor.display())
                });
            }
        }
    }
    Ok(())
}

// 未作成のパスは字句比較し、既存のパスは Unicode の別名や短縮名も実体で照合する。
fn paths_overlap(left: &Path, right: &Path) -> Result<bool> {
    if left.components().zip(right.components()).all(|(a, b)| {
        a.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
    }) {
        return Ok(true);
    }
    for (path, target) in [(left, right), (right, left)] {
        for ancestor in path.ancestors() {
            match same_file::is_same_file(ancestor, target) {
                Ok(true) => return Ok(true),
                Ok(false) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).context("Failed to compare input and output paths");
                }
            }
        }
    }
    Ok(false)
}

fn validate_dist(config: &Config, config_path: &Path) -> Result<()> {
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let dist = site_directory(&root, &config.dist, "dist")?;
    for input in [&config.r#static, &config.layouts, PARTIALS_DIR, ".git"] {
        let input = absolute_path(&root, Path::new(input));
        anyhow::ensure!(
            !paths_overlap(&dist, &input)?,
            "dist must not overlap input or Git directories: {}",
            input.display()
        );
    }
    anyhow::ensure!(
        !paths_overlap(&dist, &absolute_path(&root, config_path))?,
        "dist must not contain the configuration file"
    );
    // Git のメタデータは入れ子でも削除しない。Windows の大小文字も考慮する。
    anyhow::ensure!(
        !dist
            .strip_prefix(&root)?
            .iter()
            .any(|part| part.to_string_lossy().eq_ignore_ascii_case(".git")),
        "dist must not contain Git directories"
    );
    ensure_no_symlink_ancestors(&dist, "dist")
}

/// Frontmatter とコンテンツを解析する
fn parse_frontmatter(content: &str) -> Result<(FrontMatter, String)> {
    // 区切りの抽出では YAML ツリーを作らず、型に直接デシリアライズする。
    struct RawFrontMatter;
    impl gray_matter::engine::Engine for RawFrontMatter {
        fn parse(_: &str) -> gray_matter::Pod {
            gray_matter::Pod::Null
        }
    }

    if let Some((first_line, rest)) = content.split_once('\n')
        && first_line.trim_end() == "---"
    {
        let mut bytes = 0;
        for line in rest.split_inclusive('\n') {
            if line.trim_end() == "---" {
                break;
            }
            bytes += line.len();
            anyhow::ensure!(
                bytes <= MAX_FRONTMATTER_BYTES,
                "Front matter exceeds {MAX_FRONTMATTER_BYTES} bytes"
            );
        }
    }
    let matter = gray_matter::Matter::<RawFrontMatter>::new();
    let parsed = matter.parse(content);

    let frontmatter = if parsed.matter.is_empty() {
        FrontMatter::default()
    } else {
        serde_yaml::from_str(&parsed.matter).context("Invalid YAML front matter")?
    };

    Ok((frontmatter, parsed.content))
}

/// OGP 画像の URL を解決する。未指定時は空文字、base_url 未指定時はルート相対にする。
fn resolve_og_image_url(image: &str, base_url: &str) -> Result<String> {
    anyhow::ensure!(
        !image.chars().any(char::is_control),
        "og_image must not contain control characters"
    );
    let image = image.trim();
    if image.is_empty() {
        return Ok(String::new());
    }
    anyhow::ensure!(
        !image.starts_with("//") && !image.contains('\\'),
        "og_image must be an HTTP(S) URL or a site image path"
    );
    match url::Url::parse(image) {
        Ok(url) => {
            anyhow::ensure!(
                matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
                "og_image must use HTTP or HTTPS"
            );
            return Ok(url.to_string());
        }
        Err(url::ParseError::RelativeUrlWithoutBase) => {}
        Err(error) => return Err(error).context("Invalid og_image URL"),
    }

    // base_url をディレクトリとして扱い、画像パスを標準の URL 規則で解決する。
    // 未指定時だけ仮のオリジンを使い、返す値からは取り除く。
    let base = if base_url.is_empty() {
        "https://vss.invalid/".to_string()
    } else {
        format!("{}/", base_url.trim_end_matches('/'))
    };
    let base = url::Url::parse(&base).context("Invalid base_url for og_image")?;
    anyhow::ensure!(
        matches!(base.scheme(), "http" | "https")
            && base.host_str().is_some()
            && base.query().is_none()
            && base.fragment().is_none(),
        "base_url for og_image must be an HTTP(S) URL without a query or fragment"
    );
    let resolved = base.join(image).context("Failed to resolve og_image URL")?;
    if base_url.is_empty() {
        Ok(resolved[url::Position::BeforePath..].to_string())
    } else {
        Ok(resolved.to_string())
    }
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

/// ページとして公開しない Markdown 部品を読み込む。
fn load_markdown_partials(paths: &[PathBuf], config: &Config) -> Result<HashMap<String, String>> {
    let mut partials = HashMap::new();
    for path in paths {
        let content = read_source_file(path)
            .with_context(|| format!("Failed to read markdown partial: {}", path.display()))?;
        let (_, markdown_content) = parse_frontmatter(&content)
            .with_context(|| format!("Failed to parse markdown partial: {}", path.display()))?;
        let html = markdown_to_html(
            &markdown_content,
            config.build.markdown.allow_dangerous_html,
        )
        .with_context(|| format!("Failed to convert markdown partial: {}", path.display()))?;
        let relative_path = path.strip_prefix(PARTIALS_DIR)?.with_extension("");
        // テンプレート内では OS によらず / を区切り文字にする。
        let name = relative_path
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        partials.insert(format!("partials.{name}"), html);
    }
    Ok(partials)
}

/// テンプレートを読み込んでキャッシュする
fn load_templates(layouts_dir: &str) -> Result<HashMap<String, ramhorns::Template<'static>>> {
    let mut templates = HashMap::new();
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let layouts_path = absolute_path(&root, Path::new(layouts_dir));
    for path in source_files(&layouts_path, Some("html"), &[], false)? {
        let template_content = read_source_file(&path)
            .with_context(|| format!("Failed to read template: {}", path.display()))?;
        let template = ramhorns::Template::new(template_content)
            .with_context(|| format!("Failed to parse template: {}", path.display()))?;

        // layouts/ からの相対パスをキーとする
        if let Ok(rel_path) = path.strip_prefix(&layouts_path) {
            let key = rel_path.to_string_lossy().to_string();
            templates.insert(key, template);
        }
    }

    Ok(templates)
}

/// Markdown が使用するテンプレートのキーを優先順位に従って検索する。
fn determine_template_key<'a>(
    templates: &'a HashMap<String, ramhorns::Template<'static>>,
    html_path: &str,
) -> Option<&'a str> {
    if let Some((key, _)) = templates.get_key_value(html_path) {
        return Some(key);
    }
    if let Some(dir) = Path::new(html_path).parent() {
        let key = format!("{}/default.html", dir.display());
        if let Some((key, _)) = templates.get_key_value(&key) {
            return Some(key);
        }
    }
    templates
        .get_key_value("default.html")
        .map(|(key, _)| key.as_str())
}

fn lookup_template<'a>(
    templates: &'a HashMap<String, ramhorns::Template<'static>>,
    html_path: &str,
) -> Option<&'a ramhorns::Template<'static>> {
    determine_template_key(templates, html_path).and_then(|key| templates.get(key))
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
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let static_path = absolute_path(&root, Path::new(static_dir));
    if !static_path.exists() {
        // static ディレクトリがなければスキップ
        return Ok(());
    }

    for src_path in static_source_files(&static_path)? {
        // static/ からの相対パスを取得
        if let Ok(rel_path) = src_path.strip_prefix(&static_path) {
            let dest_path = Path::new(dist_dir).join(rel_path);

            // 親ディレクトリを作成
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            // ファイルをコピー
            ensure_source_path(&src_path)?;
            fs::copy(&src_path, &dest_path).with_context(|| {
                format!(
                    "Failed to copy file from {} to {}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;
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

    let (partial_files, md_files) = find_markdown_sources(&config)?;
    let partials = load_markdown_partials(&partial_files, &config)?;

    // 7. テンプレートを読み込む
    let templates = load_templates(&config.layouts)?;

    // 8. 投稿メタデータを収集
    let mut all_posts: Vec<PostMetadata> = Vec::new();

    // 9. 各 Markdown ファイルを処理
    for md_path in md_files {
        let post_metadata = process_markdown_file(&md_path, &config, &templates, &partials)?;
        // タグページ生成が有効な場合のみメタデータを収集
        if config.build.tags.enable
            && let Some(metadata) = post_metadata
        {
            all_posts.push(metadata);
        }
    }

    // 10. タグページを生成
    if !all_posts.is_empty() {
        generate_tag_pages(all_posts, &config, &templates, &partials)?;
    }

    Ok(())
}

/// 個別の Markdown ファイルを処理する
fn process_markdown_file(
    md_path: &Path,
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
    partials: &HashMap<String, String>,
) -> Result<Option<PostMetadata>> {
    // Markdown ファイルを読み込む
    let content = read_source_file(md_path)
        .with_context(|| format!("Failed to read markdown file: {}", md_path.display()))?;

    // Frontmatter を解析
    let (frontmatter, markdown_content) = parse_frontmatter(&content)
        .with_context(|| format!("Failed to parse markdown file: {}", md_path.display()))?;

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
                .filter(|tag_name| is_safe_tag_name(tag_name))
                .map(|tag_name| Tag {
                    name: tag_name.clone(),
                    url: tag_url(config, tag_name),
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
        og_image: frontmatter.og_image.clone(),
        og_image_url: resolve_og_image_url(&frontmatter.og_image, &config.base_url)
            .with_context(|| format!("Invalid og_image in {}", md_path.display()))?,
        has_tags,
        tags,
        partials,
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

    Ok(post_metadata(frontmatter, &html_path))
}

fn post_metadata(frontmatter: FrontMatter, html_path: &Path) -> Option<PostMetadata> {
    let tags = frontmatter.tags.filter(|tags| !tags.is_empty())?;
    Some(PostMetadata {
        title: frontmatter.title,
        description: frontmatter.description,
        author: frontmatter.author,
        pub_datetime: frontmatter.pub_datetime,
        url: format!("/{}", html_path.to_string_lossy()),
        tags: Some(tags),
    })
}

/// タグページを生成する（設定を考慮）
fn generate_tag_pages(
    all_posts: Vec<PostMetadata>,
    config: &Config,
    templates: &HashMap<String, ramhorns::Template<'static>>,
    partials: &HashMap<String, String>,
) -> Result<()> {
    // タグページ生成が無効な場合は何もしない
    if !config.build.tags.enable {
        println!("Tag page generation is disabled in config");
        return Ok(());
    }

    // タグごとに投稿をグループ化
    let mut tag_to_posts: HashMap<&str, Vec<&PostMetadata>> = HashMap::new();

    for post in &all_posts {
        if let Some(tags_vec) = &post.tags {
            for tag_name in tags_vec {
                if !is_safe_tag_name(tag_name) {
                    continue;
                }
                tag_to_posts
                    .entry(tag_name.as_str())
                    .or_default()
                    .push(post);
            }
        }
    }

    // 各タグのページを生成
    for (tag_name, posts) in tag_to_posts {
        let context = TagPageContext {
            site_title: config.site_title.clone(),
            site_description: config.site_description.clone(),
            base_url: config.base_url.clone(),
            tag_name: tag_name.to_string(),
            posts,
            partials,
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

        let output_path = tag_output_path(config, tag_name);

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

fn has_symlink_component(path: &Path) -> Result<bool> {
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        if fs::symlink_metadata(ancestor)
            .with_context(|| format!("Failed to inspect source path: {}", ancestor.display()))?
            .file_type()
            .is_symlink()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_source_path(path: &Path) -> Result<()> {
    let root = std::env::current_dir().context("Failed to get current directory")?;
    anyhow::ensure!(
        !has_symlink_component(&absolute_path(&root, path))?,
        "Symbolic links are not supported in source paths: {}",
        path.display()
    );
    Ok(())
}

fn read_source_file(path: &Path) -> Result<String> {
    ensure_source_path(path)?;
    Ok(fs::read_to_string(path)?)
}

/// リンク先のディレクトリへ降りず、通常ファイルだけを列挙する。
fn source_files(
    directory: &Path,
    extension: Option<&str>,
    excluded: &[PathBuf],
    skip_dot_directories: bool,
) -> Result<Vec<PathBuf>> {
    if !directory.try_exists()? || has_symlink_component(directory)? {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(directory)
        .follow_links(false)
        .follow_root_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            // 探索ルートの名前にかかわらず、配下のドットディレクトリだけを除外する。
            let excluded_dot_directory = skip_dot_directories
                && entry.depth() > 0
                && entry.file_type().is_dir()
                && entry.file_name().to_string_lossy().starts_with('.');
            !excluded_dot_directory && !excluded.iter().any(|path| entry.path().starts_with(path))
        })
    {
        let entry = entry.context("Failed to enumerate source files")?;
        if entry.file_type().is_file()
            && extension
                .is_none_or(|ext| entry.path().extension().is_some_and(|value| value == ext))
        {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

/// 存在しないパスも扱えるように、字句的に絶対パスへ正規化する。
pub(crate) fn absolute_path(root: &Path, path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in root.join(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn site_relative_path(root: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = absolute_path(root, path);
    let relative = absolute
        .strip_prefix(root)
        .with_context(|| format!("Changed path is outside the site: {}", path.display()))?;
    Ok(relative.to_path_buf())
}

fn find_markdown_sources(config: &Config) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let dist = absolute_path(&root, Path::new(&config.dist));
    Ok(source_files(&root, Some("md"), &[dist], true)?
        .into_iter()
        .map(|path| path.strip_prefix(&root).unwrap().to_path_buf())
        .filter(|path| {
            !config
                .build
                .ignore_files
                .iter()
                .any(|ignore| path.to_string_lossy().contains(ignore))
        })
        .partition(|path| path.starts_with(PARTIALS_DIR)))
}

fn collect_post_metadata(paths: &[PathBuf]) -> Result<Vec<PostMetadata>> {
    let mut posts = Vec::new();
    for path in paths {
        let content = read_source_file(path)
            .with_context(|| format!("Failed to read markdown file: {}", path.display()))?;
        let (frontmatter, _) = parse_frontmatter(&content)
            .with_context(|| format!("Failed to parse markdown file: {}", path.display()))?;
        if let Some(post) = post_metadata(frontmatter, &path.with_extension("html")) {
            posts.push(post);
        }
    }
    Ok(posts)
}

fn tag_url(config: &Config, tag: &str) -> String {
    let path = config.build.tags.url_pattern.replace("{tag}", tag);
    let encoded = path
        .trim_start_matches('/')
        .split('/')
        .map(|segment| utf8_percent_encode(segment, URL_SEGMENT_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/");
    format!("/{encoded}")
}

fn tag_output_path(config: &Config, tag: &str) -> PathBuf {
    let path = config.build.tags.url_pattern.replace("{tag}", tag);
    let relative = path.trim_start_matches('/');
    let output = Path::new(&config.dist).join(relative);
    if relative.ends_with('/') {
        output.join("index.html")
    } else {
        output
    }
}

fn tag_output_paths(config: &Config, posts: &[PostMetadata]) -> HashSet<PathBuf> {
    if !config.build.tags.enable {
        return HashSet::new();
    }
    posts
        .iter()
        .flat_map(|post| post.tags.iter().flatten())
        .filter(|tag| is_safe_tag_name(tag))
        .map(|tag| tag_output_path(config, tag))
        .collect()
}

/// 静的ファイルでは公開用の .well-known 以外の隠しパスを除外する。
fn is_public_static_path(relative: &Path) -> bool {
    relative.iter().enumerate().all(|(index, part)| {
        let name = part.to_string_lossy();
        !name.starts_with('.') || (index == 0 && name == ".well-known")
    })
}

fn static_source_files(directory: &Path) -> Result<Vec<PathBuf>> {
    Ok(source_files(directory, None, &[], false)?
        .into_iter()
        .filter(|path| {
            path.strip_prefix(directory)
                .is_ok_and(is_public_static_path)
        })
        .collect())
}

fn input_paths(
    config: &Config,
    partials: &[PathBuf],
    pages: &[PathBuf],
) -> Result<HashSet<PathBuf>> {
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let mut paths: HashSet<_> = partials
        .iter()
        .chain(pages)
        .map(|path| absolute_path(&root, path))
        .collect();
    for (directory, extension) in [(&config.r#static, None), (&config.layouts, Some("html"))] {
        let directory = absolute_path(&root, Path::new(directory));
        paths.extend(if extension.is_none() {
            static_source_files(&directory)?
        } else {
            source_files(&directory, extension, &[], false)?
        });
    }
    Ok(paths)
}

/// 削除済みのタグ出力を把握し、必要なときだけ全体を再生成する。
pub(crate) struct IncrementalBuilder {
    config_path: PathBuf,
    pub(crate) config: Config,
    tag_outputs: HashSet<PathBuf>,
    needs_full_build: bool,
    inputs: HashSet<PathBuf>,
}

impl IncrementalBuilder {
    pub(crate) fn new(config_path: &Path) -> Result<Self> {
        let config = load_config(config_path)?;
        let (partials, pages) = find_markdown_sources(&config)?;
        let inputs = input_paths(&config, &partials, &pages)?;
        let posts = collect_post_metadata(&pages)?;
        let tag_outputs = tag_output_paths(&config, &posts);
        Ok(Self {
            config_path: config_path.to_path_buf(),
            config,
            tag_outputs,
            needs_full_build: false,
            inputs,
        })
    }

    pub(crate) fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub(crate) fn has_missing_inputs(&self) -> bool {
        self.inputs
            .iter()
            .any(|path| !path.is_file() || has_symlink_component(path).unwrap_or(true))
    }

    pub(crate) fn rebuild(&mut self, changed_files: &[ChangedFile]) -> Result<()> {
        if changed_files.is_empty() {
            return Ok(());
        }
        let result = self.rebuild_changed(changed_files);
        // 失敗前に書き込まれた出力や失われた出力も、次回まとめて復旧する。
        self.needs_full_build = result.is_err();
        result
    }

    fn rebuild_changed(&mut self, changed_files: &[ChangedFile]) -> Result<()> {
        let root = std::env::current_dir().context("Failed to get current directory")?;
        let config = load_config(&self.config_path)?;
        let static_dir = absolute_path(&root, Path::new(&config.r#static));
        let layouts_dir = absolute_path(&root, Path::new(&config.layouts));
        let dist_dir = absolute_path(&root, Path::new(&config.dist));
        // 全体コピーと増分コピーで同じ公開範囲を使う。削除通知も同様に扱う。
        let public_changes: Vec<_> = changed_files
            .iter()
            .filter(|changed| match changed {
                ChangedFile::Static(path) | ChangedFile::Deleted(path) => {
                    absolute_path(&root, path)
                        .strip_prefix(&static_dir)
                        .map(is_public_static_path)
                        .unwrap_or(true)
                }
                _ => true,
            })
            .cloned()
            .collect();
        let changed_files = public_changes.as_slice();
        let force_full_build = self.needs_full_build
            || self.has_missing_inputs()
            || changed_files.contains(&ChangedFile::Config);
        let mut full_build = force_full_build;
        let (partial_files, pages) = find_markdown_sources(&config)?;
        let inputs = input_paths(&config, &partial_files, &pages)?;

        // 書き込みを始める前に、通知に含まれる全パスを検証する。
        for changed in changed_files {
            let path = match changed {
                ChangedFile::Config => {
                    full_build = true;
                    continue;
                }
                ChangedFile::Markdown(path)
                | ChangedFile::Static(path)
                | ChangedFile::Template(path)
                | ChangedFile::Deleted(path) => path,
            };
            let relative = site_relative_path(&root, path)?;
            let absolute = absolute_path(&root, path);
            // 同じ通知バッチには、変更前の設定で分類されたパスも含まれる。
            if force_full_build {
                continue;
            }
            anyhow::ensure!(
                !absolute.starts_with(&dist_dir),
                "Changed path is inside the output directory"
            );
            if !matches!(changed, ChangedFile::Deleted(_)) && has_symlink_component(&absolute)? {
                full_build = true;
                continue;
            }
            if absolute.is_dir() || relative.starts_with(PARTIALS_DIR) {
                full_build = true;
            }
            if matches!(changed, ChangedFile::Static(_) | ChangedFile::Template(_))
                && pages.contains(&relative)
            {
                full_build = true;
            }
            if matches!(changed, ChangedFile::Static(_) | ChangedFile::Deleted(_))
                && let Ok(relative_static) = absolute.strip_prefix(&static_dir)
            {
                let output = dist_dir.join(relative_static);
                // 全体ビルドと同様に、静的コピーより生成ページを優先する。
                if pages
                    .iter()
                    .any(|page| dist_dir.join(page.with_extension("html")) == output)
                    || self
                        .tag_outputs
                        .iter()
                        .any(|tag| absolute_path(&root, tag) == output)
                {
                    full_build = true;
                }
            }
            match changed {
                ChangedFile::Deleted(_) if path.extension().is_some_and(|ext| ext == "md") => {
                    full_build = true;
                }
                ChangedFile::Static(_) => {
                    anyhow::ensure!(
                        absolute.starts_with(&static_dir),
                        "Static path is outside the static directory"
                    );
                }
                ChangedFile::Template(_) => {
                    anyhow::ensure!(
                        absolute.starts_with(&layouts_dir),
                        "Template path is outside the layouts directory"
                    );
                }
                ChangedFile::Deleted(_) if !absolute.starts_with(&static_dir) => full_build = true,
                ChangedFile::Deleted(_)
                    if !Path::new(&config.dist)
                        .join(absolute.strip_prefix(&static_dir)?)
                        .is_file() =>
                {
                    full_build = true
                }
                _ => {}
            }
        }

        let needs_pages = full_build
            || changed_files
                .iter()
                .any(|file| matches!(file, ChangedFile::Markdown(_) | ChangedFile::Template(_)));
        if !needs_pages {
            for changed in changed_files {
                let (path, deleted) = match changed {
                    ChangedFile::Static(path) => (path, false),
                    ChangedFile::Deleted(path) => (path, true),
                    _ => continue,
                };
                let source = absolute_path(&root, path);
                let output = Path::new(&config.dist).join(source.strip_prefix(&static_dir)?);
                if deleted {
                    if output.is_file() {
                        fs::remove_file(&output)?;
                    }
                } else {
                    copy_static_file(&source, &output)?;
                }
            }
            self.config = config;
            self.inputs = inputs;
            return Ok(());
        }

        let posts = collect_post_metadata(&pages)?;
        let tag_outputs = tag_output_paths(&config, &posts);
        // 最後の投稿が消えたタグは、任意の URL パターンでも残さない。
        full_build |= !self.tag_outputs.is_subset(&tag_outputs);
        if full_build {
            println!("[INFO] Running full rebuild...");
            run_build(&self.config_path)?;
        } else {
            let templates = load_templates(&config.layouts)?;
            let partials = load_markdown_partials(&partial_files, &config)?;
            let mut affected_pages = HashSet::new();
            for changed in changed_files {
                match changed {
                    ChangedFile::Markdown(path) => {
                        let relative = site_relative_path(&root, path)?;
                        if pages.contains(&relative) {
                            affected_pages.insert(relative);
                        }
                    }
                    ChangedFile::Template(path) => {
                        let absolute = absolute_path(&root, path);
                        let key = absolute.strip_prefix(&layouts_dir)?.to_string_lossy();
                        for page in &pages {
                            let html = page.with_extension("html");
                            if determine_template_key(&templates, &html.to_string_lossy())
                                == Some(key.as_ref())
                            {
                                affected_pages.insert(page.clone());
                            }
                        }
                    }
                    ChangedFile::Static(path) => {
                        let source = absolute_path(&root, path);
                        let output =
                            Path::new(&config.dist).join(source.strip_prefix(&static_dir)?);
                        copy_static_file(&source, &output)?;
                    }
                    ChangedFile::Deleted(path) => {
                        let source = absolute_path(&root, path);
                        let output =
                            Path::new(&config.dist).join(source.strip_prefix(&static_dir)?);
                        if output.is_file() {
                            fs::remove_file(output)?;
                        }
                    }
                    ChangedFile::Config => unreachable!(),
                }
            }
            for page in affected_pages {
                process_markdown_file(&page, &config, &templates, &partials)?;
            }
            if !posts.is_empty() {
                generate_tag_pages(posts, &config, &templates, &partials)?;
            }
        }
        self.config = config;
        self.tag_outputs = tag_outputs;
        self.inputs = inputs;
        Ok(())
    }
}

fn copy_static_file(source: &Path, output: &Path) -> Result<()> {
    ensure_source_path(source)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, output)
        .with_context(|| format!("Failed to copy static file: {}", source.display()))?;
    println!("Copied: {}", output.display());
    Ok(())
}
