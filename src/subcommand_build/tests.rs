use super::*;
use glob::glob;
use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};
use tempfile::TempDir;

static CWD: Mutex<()> = Mutex::new(());

mod publication;

#[test]
fn site_relative_paths_accept_canonical_prefixes_and_missing_files() {
    let _site = Site::new();
    let root = std::env::current_dir().unwrap();
    let canonical = root.canonicalize().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for base in [&root, &canonical] {
        for (path, expected) in [
            (root.join("index.md"), "index.md"),
            (canonical.join("index.md"), "index.md"),
            (canonical.join("deleted.md"), "deleted.md"),
        ] {
            assert_eq!(
                site_relative_path(base, &path).unwrap(),
                Path::new(expected)
            );
        }
        assert!(site_relative_path(base, &root.parent().unwrap().join("outside.md")).is_err());
        assert!(
            site_relative_path(
                base,
                &outside.path().canonicalize().unwrap().join("index.md")
            )
            .is_err()
        );
    }
}

#[test]
fn template_lookups_use_portable_nested_paths() {
    let templates = HashMap::from([
        (
            "nested/page.html".to_string(),
            ramhorns::Template::new("page").unwrap(),
        ),
        (
            "nested/default.html".to_string(),
            ramhorns::Template::new("default").unwrap(),
        ),
    ]);
    for (file, expected) in [
        ("page.html", "nested/page.html"),
        ("other.html", "nested/default.html"),
    ] {
        let native = Path::new("nested").join(file);
        assert_eq!(
            determine_template_key(&templates, &native.to_string_lossy()),
            Some(expected)
        );
    }
    assert_eq!(
        template_path_key(&Path::new("nested").join("page.html")),
        "nested/page.html"
    );

    let _site = Site::new();
    let loaded = load_templates("layouts").unwrap();
    assert!(loaded.contains_key("tags/default.html"));
    assert_eq!(
        determine_template_key(
            &loaded,
            &Path::new("tags").join("default.html").to_string_lossy()
        ),
        Some("tags/default.html")
    );
}

#[test]
fn rejects_frontmatter_resource_amplification_before_building_pages() {
    let too_large = format!("---\ntitle: {}\n---\nHome", "x".repeat(65_536));
    assert!(parse_frontmatter(&too_large).is_err());
    let too_many_tags = format!("---\ntags: [{}]\n---\nHome", vec!["news"; 129].join(","));
    assert!(parse_frontmatter(&too_many_tags).is_err());
    let long_tag = format!("---\ntags: ['{}']\n---\nHome", "x".repeat(257));
    assert!(parse_frontmatter(&long_tag).is_err());
    let nested_aliases = "---\na: &a [x, x, x, x]\nb: &b [*a, *a, *a, *a]\nc: &c [*b, *b, *b, *b]\ntags: [*c, *c, *c, *c]\n---\nHome";
    assert!(parse_frontmatter(nested_aliases).is_err());
    let aliased_tags = format!(
        "---\nname: &name news\ntags: [{}]\n---\nHome",
        vec!["*name"; 129].join(",")
    );
    assert!(parse_frontmatter(&aliased_tags).is_err());
}

#[test]
fn frontmatter_limits_accept_boundaries_and_preserve_plain_markdown() {
    let boundary = format!(
        "---\ntitle: {}\n---\nHome",
        "x".repeat(MAX_FRONTMATTER_BYTES - 8)
    );
    assert!(parse_frontmatter(&boundary).is_ok());
    let too_much_whitespace = format!("---\n{}\n---\nHome", " ".repeat(MAX_FRONTMATTER_BYTES));
    assert!(parse_frontmatter(&too_much_whitespace).is_err());
    let tags = vec!["x".repeat(MAX_TAG_BYTES); MAX_TAGS].join(",");
    let (metadata, content) =
        parse_frontmatter(&format!("---\ntags: [{tags}]\n---\nHome")).unwrap();
    assert_eq!(metadata.tags.unwrap().len(), MAX_TAGS);
    assert_eq!(content, "Home");
    let plain = "x".repeat(MAX_FRONTMATTER_BYTES + 1);
    assert_eq!(parse_frontmatter(&plain).unwrap().1, plain);
    assert!(
        parse_frontmatter("---\ntags: null\n---\nHome")
            .unwrap()
            .0
            .tags
            .is_none()
    );
    assert!(
        parse_frontmatter("---\ntags: []\n---\nHome")
            .unwrap()
            .0
            .tags
            .unwrap()
            .is_empty()
    );
}

#[test]
fn parses_typed_frontmatter_without_expanding_unused_alias_trees() {
    let mut input =
        String::from("---\ntitle: &title Home\nauthor: *title\na0: &a0 [x,x,x,x,x,x,x,x,x,x]\n");
    for level in 1..=4 {
        input.push_str(&format!(
            "a{level}: &a{level} [{}]\n",
            vec![format!("*a{}", level - 1); 10].join(",")
        ));
    }
    input.push_str("tags: [news]\n---\n# Home");
    let (metadata, contents) = parse_frontmatter(&input.replace('\n', "\r\n")).unwrap();
    assert_eq!(metadata.title, "Home");
    assert_eq!(metadata.author, "Home");
    assert_eq!(metadata.tags.unwrap(), ["news"]);
    assert_eq!(contents, "# Home");
    assert!(parse_frontmatter("---\ntitle: &a [*a]\n---\nHome").is_err());
    assert!(parse_frontmatter("---\ntags: [\n---\nHome").is_err());
}

#[test]
fn rejects_excessive_tags_in_incremental_pages_and_partials() {
    let site = Site::new();
    let mut builder = site.builder();
    let tags = format!(
        "---\ntags: [{}]\n---\nHome",
        vec!["news"; MAX_TAGS + 1].join(",")
    );
    write("index.md", &tags);
    assert!(
        builder
            .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
            .is_err()
    );
    write("index.md", "# Recovered");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    write("partials/news.md", &tags);
    assert!(run_build(Path::new("vss.toml")).is_err());
}

#[test]
fn renders_all_allowed_tag_pages_with_post_fields() {
    let _site = Site::new();
    let tags: Vec<_> = (0..MAX_TAGS).map(|n| format!("tag-{n}")).collect();
    write(
        "index.md",
        &format!("---\ntitle: Tagged\ntags: [{}]\n---\nHome", tags.join(",")),
    );
    write(
        "layouts/tags/default.html",
        "{{#posts}}{{title}}|{{url}}|{{#tags}}x{{/tags}}{{/posts}}",
    );
    run_build(Path::new("vss.toml")).unwrap();
    for tag in &tags {
        let page = read(&format!("dist/tags/{tag}/index.html"));
        assert_eq!(page, format!("Tagged|/index.html|{}", "x".repeat(MAX_TAGS)));
    }
}

#[test]
fn rejects_tag_names_that_can_be_url_schemes_or_windows_prefixes() {
    for tag in [
        "javascript:alert(1)",
        "C:escape",
        "data:text/html,test",
        ".",
        "a\nb",
    ] {
        assert!(!is_safe_tag_name(tag), "Accepted unsafe tag: {tag:?}");
    }
}

#[test]
fn excludes_nonportable_tag_filenames_without_breaking_the_build() {
    let _site = Site::new();
    let invalid = [
        "a?b",
        "a*b",
        "a<b",
        "a>b",
        "a|b",
        "a\"b",
        "trailing.",
        "trailing ",
        "CON",
        "con.txt",
        "PRN",
        "AUX",
        "NUL",
        "COM1",
        "LPT9",
        "COM¹",
        "LPT²",
        "CONIN$",
        "CONOUT$",
    ];
    for tag in invalid {
        assert!(!is_safe_tag_name(tag), "accepted {tag:?}");
    }
    let tags = invalid
        .iter()
        .map(|tag| format!("'{tag}'"))
        .chain([
            "'news'".into(),
            "'日本語'".into(),
            "'C++'".into(),
            "'a b'".into(),
        ])
        .collect::<Vec<String>>()
        .join(",");
    write("index.md", &format!("---\ntags: [{tags}]\n---\nHome"));
    write(
        "layouts/default.html",
        "{{#tags}}<a href=\"{{url}}\">{{name}}</a>{{/tags}}",
    );
    run_build(Path::new("vss.toml")).unwrap();
    let page = read("dist/index.html");
    for (name, url) in [
        ("news", "/tags/news/"),
        ("日本語", "/tags/%E6%97%A5%E6%9C%AC%E8%AA%9E/"),
        ("C++", "/tags/C%2B%2B/"),
        ("a b", "/tags/a%20b/"),
    ] {
        assert!(page.contains(&format!("<a href=\"{url}\">{name}</a>")));
        assert!(Path::new(&format!("dist/tags/{name}/index.html")).is_file());
    }
    assert_eq!(fs::read_dir("dist/tags").unwrap().count(), 4);
}

#[test]
fn incremental_build_removes_tag_pages_when_tags_become_nonportable() {
    let site = Site::new();
    let mut builder = site.builder();
    write("index.md", "---\ntags: ['a?b', CON, 'C++']\n---\nChanged");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert!(!Path::new("dist/tags/news").exists());
    assert_eq!(fs::read_dir("dist/tags").unwrap().count(), 1);
    assert!(Path::new("dist/tags/C++/index.html").is_file());
    matches_full_build("dist");
}

#[test]
fn relative_tag_patterns_cannot_publish_javascript_links() {
    let _site = Site::new();
    write("vss.toml", "[build.tags]\nurl_pattern = '{tag}.html'");
    write(
        "layouts/default.html",
        "{{#tags}}<a href=\"{{url}}\">{{name}}</a>{{/tags}}",
    );
    write(
        "index.md",
        "---\ntags: ['javascript:alert(1)', 'javascript:alert(1)//', news]\n---\nHome",
    );
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(read("dist/index.html"), "<a href=\"/news.html\">news</a>");
    assert!(Path::new("dist/news.html").exists());
    assert!(!Path::new("dist/javascript:alert(1).html").exists());
}

#[test]
fn tag_urls_encode_path_characters_and_keep_output_paths_consistent() {
    let _site = Site::new();
    let config = load_config(Path::new("vss.toml")).unwrap();
    for (tag, encoded) in [
        ("C++", "C%2B%2B"),
        ("a b", "a%20b"),
        ("a#bc%20", "a%23bc%2520"),
        ("日本語", "%E6%97%A5%E6%9C%AC%E8%AA%9E"),
    ] {
        assert_eq!(tag_url(&config, tag), format!("/tags/{encoded}/"));
        assert_eq!(
            tag_output_path(&config, tag),
            PathBuf::from(format!("dist/tags/{tag}/index.html"))
        );
    }
    write("index.md", "---\ntags: ['C++', 'a b', '日本語']\n---\nHome");
    run_build(Path::new("vss.toml")).unwrap();
    for tag in ["C++", "a b", "日本語"] {
        assert!(tag_output_path(&config, tag).is_file());
    }
}

#[test]
fn rejects_tag_patterns_outside_the_site_before_deleting_outputs() {
    let _site = Site::new();
    write("dist/keep.txt", "keep");
    for pattern in [
        "../{tag}/",
        "/tags/../../{tag}",
        "//example.com/{tag}",
        "https://example.com/{tag}",
        "C:/{tag}",
        "tags\\{tag}",
    ] {
        write(
            "vss.toml",
            &format!("[build.tags]\nurl_pattern = '{pattern}'"),
        );
        assert!(
            run_build(Path::new("vss.toml")).is_err(),
            "Accepted pattern: {pattern}"
        );
        assert_eq!(read("dist/keep.txt"), "keep");
    }
}

struct Site {
    original: PathBuf,
    directory: TempDir,
    _lock: MutexGuard<'static, ()>,
}

impl Site {
    fn new() -> Self {
        let lock = CWD.lock().unwrap_or_else(|error| error.into_inner());
        let original = std::env::current_dir().unwrap();
        let directory = tempfile::tempdir().unwrap();
        std::env::set_current_dir(directory.path()).unwrap();
        write("vss.toml", "");
        write("index.md", "---\ntitle: Home\ntags: [news]\n---\n# Home");
        write("other.md", "# Other");
        write(
            "layouts/default.html",
            "{{title}}|{{{contents}}}|{{{partials.news}}}",
        );
        write(
            "layouts/tags/default.html",
            "{{tag_name}}:{{#posts}}{{title}};{{/posts}}{{{partials.news}}}",
        );
        write("static/style.css", "old");
        run_build(Path::new("vss.toml")).unwrap();
        Self {
            original,
            directory,
            _lock: lock,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.directory.path().canonicalize().unwrap().join(relative)
    }

    fn builder(&self) -> IncrementalBuilder {
        IncrementalBuilder::new(Path::new("./vss.toml")).unwrap()
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).unwrap();
    }
}

fn write(path: &str, content: &str) {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap()
}

fn output_tree(dist: &str) -> BTreeMap<PathBuf, Vec<u8>> {
    glob(&format!("{dist}/**/*"))
        .unwrap()
        .map(Result::unwrap)
        .filter(|path| path.is_file())
        .map(|path| {
            (
                path.strip_prefix(dist).unwrap().to_path_buf(),
                fs::read(path).unwrap(),
            )
        })
        .collect()
}

fn matches_full_build(dist: &str) {
    let incremental = output_tree(dist);
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(incremental, output_tree(dist));
}

#[test]
fn regenerates_gitignore_in_each_configured_output_without_changing_the_site_gitignore() {
    let _site = Site::new();
    write(".gitignore", "# User rules\r\n/private\r\n");
    write("static/.gitignore", "NOT THE OUTPUT RULE\n");
    for dist in ["dist", "public", "output/public"] {
        write("vss.toml", &format!("dist = '{dist}'"));
        for _ in 0..2 {
            write(&format!("{dist}/.gitignore"), "stale rule\n");
            run_build(Path::new("vss.toml")).unwrap();
            assert_eq!(read(&format!("{dist}/.gitignore")), "*\n");
            assert!(Path::new(&format!("{dist}/index.html")).is_file());
            assert_eq!(read(".gitignore"), "# User rules\r\n/private\r\n");
        }
    }
}

#[test]
fn tag_outputs_cannot_overwrite_or_traverse_the_managed_gitignore() {
    let _site = Site::new();
    for (pattern, tag) in [
        ("/{tag}", ".gitignore"),
        ("/{tag}", ".GITIGNORE"),
        ("/.gitignore/{tag}/", "news"),
    ] {
        write(
            "vss.toml",
            &format!("[build.tags]\nurl_pattern = '{pattern}'"),
        );
        write("index.md", &format!("---\ntags: ['{tag}']\n---\nHome"));
        let error = run_build(Path::new("vss.toml")).unwrap_err();
        assert!(
            error.to_string().contains("generated .gitignore"),
            "{error:#}"
        );
        assert_eq!(read("dist/.gitignore"), "*\n");
    }
}

#[test]
fn incremental_tag_outputs_cannot_overwrite_the_managed_gitignore() {
    let site = Site::new();
    write("vss.toml", "[build.tags]\nurl_pattern = '/{tag}'");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    write("index.md", "---\ntags: [news, '.gitignore']\n---\nChanged");
    let error = builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap_err();
    assert!(
        error.to_string().contains("generated .gitignore"),
        "{error:#}"
    );
    assert_eq!(read("dist/.gitignore"), "*\n");
    write("index.md", "---\ntags: [news]\n---\nRecovered");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert_eq!(read("dist/.gitignore"), "*\n");
    matches_full_build("dist");
}

#[test]
fn incremental_builds_preserve_the_output_gitignore() {
    let site = Site::new();
    let mut builder = site.builder();
    assert_eq!(read("dist/.gitignore"), "*\n");
    write("index.md", "# Updated");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert_eq!(read("dist/.gitignore"), "*\n");
    write("static/style.css", "updated");
    builder
        .rebuild(&[ChangedFile::Static(site.path("static/style.css"))])
        .unwrap();
    assert_eq!(read("dist/.gitignore"), "*\n");
    write("vss.toml", "dist = 'output/public'");
    builder.rebuild(&[ChangedFile::Config]).unwrap();
    assert_eq!(read("output/public/.gitignore"), "*\n");
    assert!(!Path::new(".gitignore").exists());
}

#[test]
fn excludes_dot_directories_from_markdown_pages_partials_and_tags() {
    let _site = Site::new();
    for path in [
        ".claude/skills/example/SKILL.md",
        ".github/notes.md",
        ".git/info/notes.md",
        "posts/.drafts/post.md",
        "partials/.drafts/news.md",
    ] {
        write(
            path,
            "---\ntitle: PRIVATE SENTINEL\ntags: [private]\n---\nPRIVATE SENTINEL",
        );
    }
    write("posts.notes/published.md", "Published post");
    write(".page.md", "A dot-prefixed file, not a directory");
    write("partials/news.md", "Public partial");
    write(
        "layouts/default.html",
        "{{{contents}}}|{{{partials.news}}}|{{{partials..drafts/news}}}",
    );
    run_build(Path::new("vss.toml")).unwrap();
    assert!(
        output_tree("dist")
            .values()
            .all(|bytes| { !String::from_utf8_lossy(bytes).contains("PRIVATE SENTINEL") })
    );
    assert!(!Path::new("dist/tags/private").exists());
    assert!(!Path::new("dist/.claude").exists());
    assert!(!Path::new("dist/posts/.drafts").exists());
    assert!(read("dist/posts.notes/published.html").contains("Published post"));
    assert!(read("dist/.page.html").contains("A dot-prefixed file"));
    assert!(read("dist/index.html").contains("Public partial"));

    // Excluded Markdown must not be read or parsed, even when it is invalid.
    fs::write(".claude/invalid.md", [0xff]).unwrap();
    fs::write("partials/.drafts/invalid.md", [0xff]).unwrap();
    run_build(Path::new("vss.toml")).unwrap();
}

#[test]
fn preserves_dot_directories_in_static_files_and_templates() {
    let site = Site::new();
    write("vss.toml", "[build.tags]\ntemplate = '.internal/tags.html'");
    write(
        "layouts/.internal/tags.html",
        "Internal template: {{tag_name}}",
    );
    write("static/.well-known/security.txt", "Contact: example");
    write(
        "static/.well-known/readme.md",
        "Copy this Markdown verbatim",
    );
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(read("dist/.well-known/security.txt"), "Contact: example");
    assert_eq!(
        read("dist/.well-known/readme.md"),
        "Copy this Markdown verbatim"
    );
    assert!(!Path::new("dist/static/.well-known/readme.html").exists());
    assert_eq!(read("dist/tags/news/index.html"), "Internal template: news");

    let mut builder = site.builder();
    write("static/.well-known/readme.md", "Updated static Markdown");
    write(
        "layouts/.internal/tags.html",
        "Updated template: {{tag_name}}",
    );
    builder
        .rebuild(&[
            ChangedFile::Static(site.path("static/.well-known/readme.md")),
            ChangedFile::Template(site.path("layouts/.internal/tags.html")),
        ])
        .unwrap();
    assert_eq!(
        read("dist/.well-known/readme.md"),
        "Updated static Markdown"
    );
    assert_eq!(read("dist/tags/news/index.html"), "Updated template: news");
    assert!(!Path::new("dist/static/.well-known/readme.html").exists());
    matches_full_build("dist");
}

#[test]
fn builds_dot_named_sites_with_configured_dot_source_directories() {
    let _site = Site::new();
    write(".site/vss.toml", "static = '.assets'\nlayouts = '.layouts'");
    write(".site/index.md", "Visible page");
    write(
        ".site/.layouts/default.html",
        "Explicit template: {{{contents}}}",
    );
    write(".site/.assets/.well-known/security.txt", "Public metadata");
    std::env::set_current_dir(".site").unwrap();
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(
        read("dist/index.html"),
        "Explicit template: <p>Visible page</p>"
    );
    assert_eq!(read("dist/.well-known/security.txt"), "Public metadata");
}

#[test]
fn incremental_build_excludes_dot_directories_and_removes_hidden_pages() {
    let site = Site::new();
    let mut builder = site.builder();
    let original = output_tree("dist");
    for path in [
        ".claude/skills/example/SKILL.md",
        "partials/.drafts/news.md",
    ] {
        write(path, "---\ntags: [private]\n---\nPRIVATE SENTINEL");
        builder
            .rebuild(&[ChangedFile::Markdown(site.path(path))])
            .unwrap();
        assert_eq!(output_tree("dist"), original);
        fs::write(path, [0xff]).unwrap();
        builder
            .rebuild(&[ChangedFile::Markdown(site.path(path))])
            .unwrap();
        assert_eq!(output_tree("dist"), original);
        fs::remove_file(path).unwrap();
        builder
            .rebuild(&[ChangedFile::Deleted(site.path(path))])
            .unwrap();
        assert_eq!(output_tree("dist"), original);
    }

    fs::create_dir(".drafts").unwrap();
    fs::rename("index.md", ".drafts/index.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Markdown(site.path(".drafts/index.md"))])
        .unwrap();
    assert!(!Path::new("dist/index.html").exists());
    assert!(!Path::new("dist/.drafts").exists());
    assert!(!Path::new("dist/tags/news/index.html").exists());
    matches_full_build("dist");

    fs::rename(".drafts/index.md", "index.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert_eq!(output_tree("dist"), original);
    matches_full_build("dist");
}

#[test]
fn edits_only_the_changed_page_and_updates_tag_metadata() {
    let site = Site::new();
    let mut builder = site.builder();
    let other = read("dist/other.html");
    write("dist/other.html", "untouched");
    write(
        "index.md",
        "---\ntitle: Updated\ntags: [news]\n---\n# Updated",
    );
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert!(read("dist/index.html").contains("Updated"));
    assert!(read("dist/tags/news/index.html").contains("Updated"));
    assert_eq!(read("dist/other.html"), "untouched");
    assert!(!Path::new("index.html").exists());
    write("dist/other.html", &other);
    matches_full_build("dist");
}

#[test]
fn refreshes_og_images_in_incremental_builds_and_after_base_url_changes() {
    let site = Site::new();
    write("vss.toml", "base_url = 'https://example.com/blog'");
    write("layouts/default.html", "{{og_image}}|{{og_image_url}}");
    write("index.md", "---\nog_image: images/old.png\n---\nHome");
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(
        read("dist/index.html"),
        "images/old.png|https://example.com/blog/images/old.png"
    );
    let mut builder = site.builder();
    write("index.md", "---\nog_image: /images/new.png\n---\nHome");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert_eq!(
        read("dist/index.html"),
        "/images/new.png|https://example.com/images/new.png"
    );
    write("vss.toml", "base_url = 'https://new.example.com'");
    builder.rebuild(&[ChangedFile::Config]).unwrap();
    assert_eq!(
        read("dist/index.html"),
        "/images/new.png|https://new.example.com/images/new.png"
    );
    write("index.md", "No image");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert_eq!(read("dist/index.html"), "|");
    matches_full_build("dist");
}

#[test]
fn incremental_static_updates_never_publish_hidden_files() {
    let site = Site::new();
    let mut builder = site.builder();
    for path in [
        ".env",
        ".git/config",
        "nested/.ssh/key",
        ".well-known/.env",
        "nested/.well-known/secret",
    ] {
        let source = format!("static/{path}");
        write(&source, "PRIVATE SENTINEL");
        builder
            .rebuild(&[ChangedFile::Static(site.path(&source))])
            .unwrap();
        assert!(!Path::new("dist").join(path).exists(), "published {path}");
        write(&source, "CHANGED PRIVATE SENTINEL");
        write("other.md", "Updated page");
        builder
            .rebuild(&[
                ChangedFile::Static(site.path(&source)),
                ChangedFile::Markdown(site.path("other.md")),
            ])
            .unwrap();
        assert!(!Path::new("dist").join(path).exists());
        fs::remove_file(&source).unwrap();
        builder
            .rebuild(&[ChangedFile::Deleted(site.path(&source))])
            .unwrap();
        matches_full_build("dist");
    }
}

#[test]
fn invalid_source_config_preserves_outputs_during_incremental_rebuild() {
    let site = Site::new();
    let mut builder = site.builder();
    let before = output_tree("dist");
    for setting in ["static", "layouts"] {
        write("vss.toml", &format!("{setting} = '../outside'"));
        assert!(builder.rebuild(&[ChangedFile::Config]).is_err());
        assert_eq!(output_tree("dist"), before);
    }
}

#[test]
fn copies_and_deletes_static_files_with_absolute_paths_and_mixed_batches() {
    let site = Site::new();
    let mut builder = site.builder();
    write("static/style.css", "new");
    write("static/nested/new.txt", "new file");
    builder
        .rebuild(&[
            ChangedFile::Static(site.path("static/style.css")),
            ChangedFile::Static(site.path("static/nested/new.txt")),
        ])
        .unwrap();
    assert_eq!(read("dist/style.css"), "new");
    assert_eq!(read("dist/nested/new.txt"), "new file");
    fs::remove_file("static/style.css").unwrap();
    write("other.md", "# Changed");
    builder
        .rebuild(&[
            ChangedFile::Deleted(site.path("static/style.css")),
            ChangedFile::Markdown(site.path("other.md")),
        ])
        .unwrap();
    assert!(!Path::new("dist/style.css").exists());
    matches_full_build("dist");
    fs::remove_dir_all("static/nested").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("static/nested"))])
        .unwrap();
    matches_full_build("dist");
}

#[test]
fn selects_template_dependents_and_handles_overrides_and_tag_templates() {
    let site = Site::new();
    write("layouts/other.html", "other template {{{contents}}}");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    let other = read("dist/other.html");
    write("dist/other.html", "untouched");
    write("layouts/default.html", "updated template {{{contents}}}");
    builder
        .rebuild(&[ChangedFile::Template(site.path("layouts/default.html"))])
        .unwrap();
    assert!(read("dist/index.html").starts_with("updated template"));
    assert_eq!(read("dist/other.html"), "untouched");
    write("dist/other.html", &other);
    matches_full_build("dist");
    write("layouts/index.html", "exact {{{contents}}}");
    builder
        .rebuild(&[ChangedFile::Template(site.path("layouts/index.html"))])
        .unwrap();
    assert!(read("dist/index.html").starts_with("exact"));
    fs::remove_file("layouts/index.html").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("layouts/index.html"))])
        .unwrap();
    write("layouts/tags/default.html", "tag updated {{tag_name}}");
    builder
        .rebuild(&[ChangedFile::Template(
            site.path("layouts/tags/default.html"),
        )])
        .unwrap();
    assert_eq!(read("dist/tags/news/index.html"), "tag updated news");
    matches_full_build("dist");
}

#[test]
fn refreshes_partial_consumers_without_publishing_partials_or_ignored_content() {
    let site = Site::new();
    write("vss.toml", "[build]\nignore_files = [\"ignored.md\"]");
    write("partials/ignored.md", "ignored");
    fs::write("partials/ignored.md", [0xff]).unwrap();
    let mut builder = site.builder();
    write("partials/news.md", "---\ntags: [secret]\n---\n**Update**");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("partials/news.md"))])
        .unwrap();
    assert!(read("dist/index.html").contains("<strong>Update</strong>"));
    assert!(read("dist/other.html").contains("<strong>Update</strong>"));
    assert!(read("dist/tags/news/index.html").contains("<strong>Update</strong>"));
    assert!(!Path::new("dist/partials").exists());
    assert!(!Path::new("dist/tags/secret").exists());
    write("other.md", "# Changed");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("other.md"))])
        .unwrap();
    matches_full_build("dist");
    fs::remove_file("partials/news.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("partials/news.md"))])
        .unwrap();
    assert!(!read("dist/index.html").contains("<strong>Update</strong>"));
    matches_full_build("dist");
}

#[test]
fn removes_stale_tag_pages_for_custom_url_patterns() {
    let site = Site::new();
    write(
        "vss.toml",
        "[build.tags]\nurl_pattern = '/topics/{tag}.html'",
    );
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    write("index.md", "---\ntags: [new]\n---\nUpdated");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert!(!Path::new("dist/topics/news.html").exists());
    assert!(Path::new("dist/topics/new.html").exists());
    write("index.md", "No tags");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert!(!Path::new("dist/topics/new.html").exists());
    matches_full_build("dist");
}

#[test]
fn handles_renamed_pages_and_deleted_directories() {
    let site = Site::new();
    write("posts.notes/old.md", "# Old");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    fs::rename("posts.notes/old.md", "posts.notes/new.md").unwrap();
    builder
        .rebuild(&[
            ChangedFile::Deleted(site.path("posts.notes/old.md")),
            ChangedFile::Markdown(site.path("posts.notes/new.md")),
        ])
        .unwrap();
    assert!(!Path::new("dist/posts.notes/old.html").exists());
    assert!(Path::new("dist/posts.notes/new.html").exists());
    fs::remove_dir_all("posts.notes").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("posts.notes"))])
        .unwrap();
    assert!(!Path::new("dist/posts.notes").exists());
    matches_full_build("dist");
}

#[test]
fn reloads_configured_source_and_output_directories() {
    let site = Site::new();
    let mut builder = site.builder();
    write("assets/a.txt", "asset");
    write("templates/default.html", "new template {{{contents}}}");
    write("templates/tags/default.html", "new tag {{tag_name}}");
    write(
        "vss.toml",
        "dist = './public'\nstatic = './assets'\nlayouts = './templates'",
    );
    builder
        .rebuild(&[
            ChangedFile::Config,
            ChangedFile::Static(site.path("static/style.css")),
            ChangedFile::Template(site.path("layouts/default.html")),
        ])
        .unwrap();
    assert_eq!(builder.config.dist, "./public");
    write("assets/a.txt", "updated asset");
    builder
        .rebuild(&[ChangedFile::Static(site.path("assets/a.txt"))])
        .unwrap();
    assert_eq!(read("public/a.txt"), "updated asset");
    write("templates/default.html", "changed template {{{contents}}}");
    builder
        .rebuild(&[ChangedFile::Template(site.path("templates/default.html"))])
        .unwrap();
    assert!(read("public/index.html").starts_with("changed template"));
    matches_full_build("public");
}

#[test]
fn propagates_read_errors_and_recovers_after_correction() {
    let site = Site::new();
    let mut builder = site.builder();
    fs::write("index.md", [0xff]).unwrap();
    assert!(
        builder
            .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
            .is_err()
    );
    write("index.md", "# Fixed");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert!(read("dist/index.html").contains("Fixed"));
    matches_full_build("dist");
}

#[test]
fn rejects_outside_paths_before_writing_or_deleting_any_output() {
    let site = Site::new();
    let outside = tempfile::tempdir().unwrap();
    let source = outside.path().join("victim.md");
    let output = source.with_extension("html");
    fs::write(&source, "outside").unwrap();
    fs::write(&output, "keep").unwrap();
    let mut builder = site.builder();
    let before = output_tree("dist");
    for event in [
        ChangedFile::Markdown(source.clone()),
        ChangedFile::Deleted(source),
        ChangedFile::Deleted(PathBuf::from("../victim.md")),
    ] {
        assert!(
            builder
                .rebuild(&[ChangedFile::Markdown(site.path("index.md")), event])
                .is_err()
        );
        assert_eq!(output_tree("dist"), before);
        assert_eq!(fs::read_to_string(&output).unwrap(), "keep");
    }
}

#[test]
fn preserves_generated_pages_when_static_outputs_overlap() {
    let site = Site::new();
    write("static/index.html", "static index");
    write("static/tags/news/index.html", "static tag");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    write("static/index.html", "changed static index");
    write("static/tags/news/index.html", "changed static tag");
    builder
        .rebuild(&[
            ChangedFile::Static(site.path("static/index.html")),
            ChangedFile::Static(site.path("static/tags/news/index.html")),
        ])
        .unwrap();
    assert!(read("dist/index.html").contains("Home"));
    assert!(read("dist/tags/news/index.html").contains("Home"));
    fs::remove_file("static/index.html").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("static/index.html"))])
        .unwrap();
    matches_full_build("dist");
}

#[test]
fn updates_markdown_inside_static_and_layout_directories() {
    let site = Site::new();
    write("static/readme.md", "# Old static");
    write("layouts/readme.md", "# Old layout");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    write("static/readme.md", "---\ntags: [static]\n---\n# New static");
    write("layouts/readme.md", "# New layout");
    builder
        .rebuild(&[
            ChangedFile::Static(site.path("static/readme.md")),
            ChangedFile::Template(site.path("layouts/readme.md")),
        ])
        .unwrap();
    assert!(read("dist/static/readme.html").contains("New static"));
    assert!(read("dist/layouts/readme.html").contains("New layout"));
    matches_full_build("dist");
    fs::remove_file("static/readme.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("static/readme.md"))])
        .unwrap();
    assert!(!Path::new("dist/static/readme.html").exists());
    assert!(!Path::new("dist/tags/static/index.html").exists());
    matches_full_build("dist");
}

#[test]
fn builds_inside_directories_with_glob_metacharacters() {
    let _site = Site::new();
    write("site[demo]/vss.toml", "");
    write("site[demo]/index.md", "# Site");
    write("site[demo]/layouts/default.html", "{{{contents}}}");
    write("site[demo]/static/a.txt", "asset");
    std::env::set_current_dir("site[demo]").unwrap();
    run_build(Path::new("vss.toml")).unwrap();
    assert!(read("dist/index.html").contains("Site"));
    assert_eq!(read("dist/a.txt"), "asset");
}

#[test]
fn restores_all_outputs_after_a_failed_full_rebuild() {
    let site = Site::new();
    let mut builder = site.builder();
    fs::create_dir_all("partials").unwrap();
    fs::write("partials/news.md", [0xff]).unwrap();
    assert!(
        builder
            .rebuild(&[ChangedFile::Markdown(site.path("partials/news.md"))])
            .is_err()
    );
    assert!(!Path::new("dist/index.html").exists());
    write("partials/news.md", "Recovered partial");
    write("other.md", "# Recovered page");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("other.md"))])
        .unwrap();
    assert!(read("dist/index.html").contains("Recovered partial"));
    matches_full_build("dist");
}

#[test]
fn skips_unsafe_tag_names_without_writing_outside_the_output_directory() {
    let site = Site::new();
    let mut builder = site.builder();
    write("escape/index.html", "keep");
    write(
        "index.md",
        "---\ntags: ['../../escape', '日本語']\n---\n# Tagged",
    );
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("index.md"))])
        .unwrap();
    assert_eq!(read("escape/index.html"), "keep");
    assert!(Path::new("dist/tags/日本語/index.html").exists());
    matches_full_build("dist");
}

#[test]
fn detects_missing_inputs_when_only_the_new_path_is_reported() {
    let site = Site::new();
    let mut builder = site.builder();
    fs::rename("index.md", "renamed.md").unwrap();
    assert!(builder.has_missing_inputs());
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("renamed.md"))])
        .unwrap();
    assert!(!Path::new("dist/index.html").exists());
    assert!(!builder.has_missing_inputs());
    fs::rename("renamed.md", "renamed.txt").unwrap();
    assert!(builder.has_missing_inputs());
    builder.rebuild(&[ChangedFile::Config]).unwrap();
    assert!(!Path::new("dist/renamed.html").exists());
    matches_full_build("dist");
}

#[cfg(unix)]
#[test]
fn does_not_publish_symlinked_files_or_directories() {
    use std::os::unix::fs::symlink;

    let _site = Site::new();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "PRIVATE SENTINEL").unwrap();
    fs::write(outside.path().join("secret.md"), "PRIVATE SENTINEL").unwrap();
    for path in ["static/leak.txt", "leak.md", "layouts/other.html"] {
        symlink(outside.path().join("secret.txt"), path).unwrap();
    }
    fs::create_dir_all("partials").unwrap();
    symlink(outside.path().join("secret.txt"), "partials/news.md").unwrap();
    symlink(outside.path(), "static/external").unwrap();
    symlink(outside.path(), "external").unwrap();
    symlink(".", "cycle").unwrap();
    symlink("missing", "broken.md").unwrap();
    run_build(Path::new("vss.toml")).unwrap();
    let outputs = output_tree("dist");
    assert!(
        outputs
            .values()
            .all(|bytes| !String::from_utf8_lossy(bytes).contains("PRIVATE SENTINEL"))
    );
    assert!(!Path::new("dist/leak.txt").exists());
    assert!(!Path::new("dist/leak.html").exists());
    assert!(!Path::new("dist/external").exists());
    assert!(!Path::new("dist/cycle").exists());
    assert!(!Path::new("dist/broken.html").exists());
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinked_source_roots_or_their_ancestors() {
    use std::os::unix::fs::symlink;

    let _site = Site::new();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(outside.path().join("assets")).unwrap();
    fs::write(outside.path().join("assets/secret.txt"), "PRIVATE SENTINEL").unwrap();
    symlink(outside.path(), "linked").unwrap();
    for directory in ["linked", "linked/assets"] {
        write("vss.toml", &format!("static = '{directory}'"));
        run_build(Path::new("vss.toml")).unwrap();
        assert!(
            output_tree("dist")
                .values()
                .all(|bytes| !String::from_utf8_lossy(bytes).contains("PRIVATE SENTINEL"))
        );
    }
}

#[cfg(unix)]
#[test]
fn incremental_build_removes_outputs_when_inputs_become_symlinks() {
    use std::os::unix::fs::symlink;

    let site = Site::new();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "PRIVATE SENTINEL").unwrap();
    let mut builder = site.builder();
    fs::remove_file("static/style.css").unwrap();
    symlink(outside.path().join("secret.txt"), "static/style.css").unwrap();
    builder
        .rebuild(&[ChangedFile::Static(site.path("static/style.css"))])
        .unwrap();
    assert!(!Path::new("dist/style.css").exists());
    fs::remove_file("other.md").unwrap();
    symlink(outside.path().join("secret.txt"), "other.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("other.md"))])
        .unwrap();
    assert!(!Path::new("dist/other.html").exists());
    symlink(outside.path().join("secret.txt"), "static/new.txt").unwrap();
    builder
        .rebuild(&[ChangedFile::Static(site.path("static/new.txt"))])
        .unwrap();
    assert!(!Path::new("dist/new.txt").exists());
    matches_full_build("dist");
}
