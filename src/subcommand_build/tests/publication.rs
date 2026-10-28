use super::*;

const LIST: &str =
    "{{#posts}}[{{title}}|{{url}}|{{pub_datetime}}]{{/posts}}{{^posts}}NO POSTS{{/posts}}";
const TAGS: &str =
    "{{#all_tags}}[{{name}}|{{url}}|{{count}}]{{/all_tags}}{{^all_tags}}NO TAGS{{/all_tags}}";

fn indexes() {
    write("layouts/index.html", LIST);
    write("tags/index.md", "# Tags");
    write("layouts/tags/index.html", TAGS);
}

fn post(path: &str, title: &str, date: &str, extra: &str) {
    write(
        path,
        &format!(
            "---\nis_post: true\ntitle: '{title}'\npub_datetime: '{date}'\n{extra}\n---\n{title} body"
        ),
    );
}

#[test]
fn draft_and_is_post_require_booleans_and_default_to_false() {
    let (frontmatter, _) = parse_frontmatter("# Plain").unwrap();
    assert!(!frontmatter.draft);
    assert!(!frontmatter.is_post);
    for key in ["draft", "is_post"] {
        for value in ["true", "false"] {
            assert!(parse_frontmatter(&format!("---\n{key}: {value}\n---\nBody")).is_ok());
        }
        for value in ["'true'", "'false'", "null", "1", "[]", "{}", "maybe"] {
            assert!(
                parse_frontmatter(&format!("---\n{key}: {value}\n---\nBody")).is_err(),
                "accepted {key}: {value}"
            );
        }
    }
}

#[test]
fn excludes_drafts_from_pages_tags_indexes_and_partials() {
    let _site = Site::new();
    indexes();
    post("posts/public.md", "Public", "2026-10-28", "tags: [news]");
    post("posts/untagged.md", "Untagged", "", "");
    post(
        "posts/draft.md",
        "PRIVATE SENTINEL",
        "2026-10-29",
        "draft: true\ntags: [news, secret]",
    );
    write("other-draft.md", "---\ndraft: true\n---\nPRIVATE SENTINEL");
    write(
        "partials/news.md",
        "---\ndraft: true\n---\nPRIVATE SENTINEL",
    );
    post(
        "partials/post.md",
        "Not an article",
        "2026-11-01",
        "tags: [partial]",
    );
    post(".hidden/post.md", "Hidden", "2026-11-01", "tags: [hidden]");
    post("ignored.md", "Ignored", "2026-11-01", "tags: [ignored]");
    write("vss.toml", "[build]\nignore_files = ['ignored.md']");
    run_build(Path::new("vss.toml")).unwrap();

    assert_eq!(
        read("dist/index.html"),
        "[Public|/posts/public.html|2026-10-28][Untagged|/posts/untagged.html|]"
    );
    assert_eq!(read("dist/tags/index.html"), "[news|/tags/news/|1]");
    for path in [
        "posts/draft.html",
        "other-draft.html",
        "tags/secret/index.html",
        "tags/partial/index.html",
    ] {
        assert!(!Path::new("dist").join(path).exists(), "published {path}");
    }
    assert!(
        output_tree("dist")
            .values()
            .all(|bytes| !String::from_utf8_lossy(bytes).contains("PRIVATE SENTINEL"))
    );
    // Existing tag pages still include tagged ordinary pages for compatibility.
    assert_eq!(read("dist/tags/news/index.html"), "news:Public;Home;");
}

#[test]
fn orders_articles_by_instant_then_url_and_places_missing_or_invalid_dates_last() {
    let _site = Site::new();
    indexes();
    for (path, title, date) in [
        ("posts/a.md", "A", "2026-10-28T09:00:00+09:00"),
        ("posts/b.md", "B", "2026-10-28T00:00:00Z"),
        ("posts/c.md", "C", "2026-10-28"),
        ("posts/later.md", "Later", "2026-10-28T00:00:00.1Z"),
        ("posts/older.md", "Older", "2026-10-27T23:59:59Z"),
        ("posts/missing.md", "Missing", ""),
        ("posts/invalid.md", "Invalid", "2026-02-30"),
    ] {
        post(path, title, date, "tags: [ordered]");
    }
    run_build(Path::new("vss.toml")).unwrap();
    let index = read("dist/index.html");
    let titles: Vec<_> = index
        .split('[')
        .skip(1)
        .map(|entry| entry.split('|').next().unwrap())
        .collect();
    assert_eq!(
        titles,
        ["Later", "A", "B", "C", "Older", "Invalid", "Missing"]
    );
    assert_eq!(
        read("dist/tags/ordered/index.html"),
        "ordered:Later;A;B;C;Older;Invalid;Missing;"
    );
}

#[test]
fn tag_index_counts_articles_once_encodes_urls_and_excludes_unsafe_names() {
    let _site = Site::new();
    indexes();
    write(
        "vss.toml",
        "[build.tags]\nurl_pattern = '/topics/{tag}.html'",
    );
    post(
        "posts/a.md",
        "A",
        "",
        "tags: ['news', 'news', 'C++', 'a b', '日本語', 'CON', '../escape']",
    );
    post("posts/b.md", "B", "", "tags: [news]");
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(
        read("dist/tags/index.html"),
        "[C++|/topics/C%2B%2B.html|1][a b|/topics/a%20b.html|1][news|/topics/news.html|2][日本語|/topics/%E6%97%A5%E6%9C%AC%E8%AA%9E.html|1]"
    );
    assert_eq!(read("dist/topics/news.html"), "news:Home;A;B;");
    assert!(Path::new("dist/topics/C++.html").exists());
    assert!(!Path::new("dist/topics/CON.html").exists());
}

#[test]
fn renders_empty_indexes_and_works_without_generated_tag_pages() {
    let site = Site::new();
    indexes();
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(read("dist/index.html"), "NO POSTS");
    assert_eq!(read("dist/tags/index.html"), "NO TAGS");
    write("vss.toml", "[build.tags]\nenable = false");
    post("posts/a.md", "A", "", "tags: [news]");
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(read("dist/index.html"), "[A|/posts/a.html|]");
    assert_eq!(read("dist/tags/index.html"), "[news|/tags/news/|1]");
    assert!(!Path::new("dist/tags/news/index.html").exists());
    let mut builder = site.builder();
    post("posts/a.md", "Changed", "", "tags: [news]");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
        .unwrap();
    assert_eq!(read("dist/index.html"), "[Changed|/posts/a.html|]");
    matches_full_build("dist");
}

#[test]
fn incremental_draft_transitions_remove_stale_pages_and_refresh_indexes() {
    let site = Site::new();
    indexes();
    post("posts/a.md", "A", "2026-10-28", "tags: [news]");
    post("posts/b.md", "B", "2026-10-27", "tags: [news]");
    post("posts/untagged.md", "Untagged", "", "");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    for extra in ["tags: [news]\ndraft: true", "tags: [news]\ndraft: false"] {
        post("posts/a.md", "A", "2026-10-28", extra);
        builder
            .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
            .unwrap();
        let draft = extra.ends_with("true");
        assert_eq!(Path::new("dist/posts/a.html").exists(), !draft);
        assert_eq!(read("dist/index.html").contains("[A|"), !draft);
        assert_eq!(read("dist/tags/news/index.html").contains("A;"), !draft);
        assert_eq!(
            read("dist/tags/index.html"),
            if draft {
                "[news|/tags/news/|1]"
            } else {
                "[news|/tags/news/|2]"
            }
        );
        matches_full_build("dist");
    }
    post("posts/untagged.md", "Untagged", "", "draft: true");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/untagged.md"))])
        .unwrap();
    assert!(!Path::new("dist/posts/untagged.html").exists());
    assert!(!read("dist/index.html").contains("Untagged"));
    matches_full_build("dist");
}

#[test]
fn incremental_metadata_changes_membership_additions_deletions_and_renames_match_full_builds() {
    let site = Site::new();
    indexes();
    post("posts/a.md", "A", "2026-10-27", "tags: [old]");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    post("posts/b.md", "B", "2026-10-28", "tags: [news]");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/b.md"))])
        .unwrap();
    assert!(read("dist/index.html").starts_with("[B|"));
    matches_full_build("dist");
    post("posts/a.md", "Updated", "2026-10-29", "tags: [news]");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
        .unwrap();
    assert!(read("dist/index.html").starts_with("[Updated|"));
    assert_eq!(read("dist/tags/index.html"), "[news|/tags/news/|2]");
    assert!(!Path::new("dist/tags/old/index.html").exists());
    matches_full_build("dist");
    write(
        "posts/a.md",
        "---\nis_post: false\ntitle: Ordinary\ntags: [news]\n---\nOrdinary body",
    );
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
        .unwrap();
    assert!(Path::new("dist/posts/a.html").exists());
    assert!(!read("dist/index.html").contains("Ordinary"));
    assert_eq!(read("dist/tags/index.html"), "[news|/tags/news/|1]");
    matches_full_build("dist");
    fs::rename("posts/b.md", "posts/renamed.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/renamed.md"))])
        .unwrap();
    assert!(!Path::new("dist/posts/b.html").exists());
    assert!(read("dist/index.html").contains("/posts/renamed.html"));
    matches_full_build("dist");
    fs::remove_file("posts/renamed.md").unwrap();
    builder
        .rebuild(&[ChangedFile::Deleted(site.path("posts/renamed.md"))])
        .unwrap();
    assert_eq!(read("dist/index.html"), "NO POSTS");
    assert_eq!(read("dist/tags/index.html"), "NO TAGS");
    matches_full_build("dist");
}

#[test]
fn article_body_edits_preserve_other_pages_but_template_edits_refresh_indexes() {
    let site = Site::new();
    indexes();
    post("posts/a.md", "A", "", "");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    write("dist/index.html", "untouched");
    post("posts/a.md", "A", "", "");
    let mut content = read("posts/a.md");
    content.push_str("\nChanged body");
    write("posts/a.md", &content);
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
        .unwrap();
    assert_eq!(read("dist/index.html"), "untouched");
    assert!(read("dist/posts/a.html").contains("Changed body"));
    write("layouts/index.html", &format!("INDEX:{LIST}"));
    builder
        .rebuild(&[ChangedFile::Template(site.path("layouts/index.html"))])
        .unwrap();
    assert_eq!(read("dist/index.html"), "INDEX:[A|/posts/a.html|]");
    matches_full_build("dist");
}

#[test]
fn draft_partials_are_removed_and_restored_in_incremental_builds() {
    let site = Site::new();
    write("partials/news.md", "Public partial");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    for draft in [true, false] {
        write(
            "partials/news.md",
            &format!("---\ndraft: {draft}\n---\nPublic partial"),
        );
        builder
            .rebuild(&[ChangedFile::Markdown(site.path("partials/news.md"))])
            .unwrap();
        assert_eq!(read("dist/index.html").contains("Public partial"), !draft);
        assert_eq!(
            read("dist/tags/news/index.html").contains("Public partial"),
            !draft
        );
        matches_full_build("dist");
    }
}

#[test]
fn article_urls_encode_segments_without_changing_output_filenames() {
    let _site = Site::new();
    indexes();
    post("posts/日本語 a#%.md", "Encoded", "", "tags: [news]");
    run_build(Path::new("vss.toml")).unwrap();
    let url = "/posts/%E6%97%A5%E6%9C%AC%E8%AA%9E%20a%23%25.html";
    assert_eq!(read("dist/index.html"), format!("[Encoded|{url}|]"));
    assert!(Path::new("dist/posts/日本語 a#%.html").exists());
    write("layouts/tags/default.html", "{{#posts}}{{url}};{{/posts}}");
    run_build(Path::new("vss.toml")).unwrap();
    assert!(read("dist/tags/news/index.html").contains(url));
}

#[test]
fn full_and_incremental_builds_restore_static_fallbacks_when_pages_become_drafts() {
    let site = Site::new();
    indexes();
    post("posts/a.md", "A", "", "tags: [only]");
    write("static/posts/a.html", "Static fallback");
    write("static/tags/only/index.html", "Static tag fallback");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    post("posts/a.md", "A", "", "tags: [only]\ndraft: true");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
        .unwrap();
    assert_eq!(read("dist/posts/a.html"), "Static fallback");
    assert_eq!(read("dist/tags/only/index.html"), "Static tag fallback");
    assert_eq!(read("dist/index.html"), "NO POSTS");
    matches_full_build("dist");
    // A full build also removes an output produced before draft was set.
    post("posts/untagged.md", "Untagged", "", "");
    run_build(Path::new("vss.toml")).unwrap();
    assert!(Path::new("dist/posts/untagged.html").exists());
    post("posts/untagged.md", "Untagged", "", "draft: true");
    run_build(Path::new("vss.toml")).unwrap();
    assert!(!Path::new("dist/posts/untagged.html").exists());
}

#[test]
fn invalid_publication_flags_fail_the_build_and_recover_without_stale_indexes() {
    let site = Site::new();
    indexes();
    post("posts/a.md", "A", "", "tags: [news]");
    run_build(Path::new("vss.toml")).unwrap();
    let mut builder = site.builder();
    write("posts/a.md", "---\ndraft: 'false'\n---\nInvalid");
    assert!(
        builder
            .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
            .is_err()
    );
    assert!(run_build(Path::new("vss.toml")).is_err());
    post("posts/a.md", "A", "", "draft: true");
    builder
        .rebuild(&[ChangedFile::Markdown(site.path("posts/a.md"))])
        .unwrap();
    assert!(!Path::new("dist/posts/a.html").exists());
    assert_eq!(read("dist/index.html"), "NO POSTS");
    assert_eq!(read("dist/tags/index.html"), "NO TAGS");
    matches_full_build("dist");
}

#[test]
fn manual_index_pages_keep_the_existing_static_and_template_precedence() {
    let site = Site::new();
    indexes();
    post("posts/a.md", "A", "", "");
    write("static/index.html", "Static index");
    write("static/tags/index.html", "Static tags");
    run_build(Path::new("vss.toml")).unwrap();
    assert_eq!(read("dist/index.html"), "[A|/posts/a.html|]");
    assert_eq!(read("dist/tags/index.html"), "NO TAGS");
    let mut builder = site.builder();
    write("static/index.html", "Changed static index");
    builder
        .rebuild(&[ChangedFile::Static(site.path("static/index.html"))])
        .unwrap();
    assert_eq!(read("dist/index.html"), "[A|/posts/a.html|]");
    matches_full_build("dist");
}
