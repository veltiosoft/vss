use super::*;
use glob::glob;
use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};
use tempfile::TempDir;

static CWD: Mutex<()> = Mutex::new(());

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
        ("a#b?c%20", "a%23b%3Fc%2520"),
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
