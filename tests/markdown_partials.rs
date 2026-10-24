use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

fn write(site: &Path, path: &str, contents: &str) {
    let path = site.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn site(template: &str) -> TempDir {
    let site = tempfile::tempdir().unwrap();
    write(site.path(), "vss.toml", "");
    write(site.path(), "index.md", "# Home\n");
    write(site.path(), "layouts/default.html", template);
    site
}

fn build(site: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_vss"))
        .arg("build")
        .current_dir(site)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read(site: &Path, path: &str) -> String {
    fs::read_to_string(site.join(path)).unwrap()
}

#[test]
fn inserts_markdown_into_pages_and_tags_without_publishing_partials() {
    let site = site(
        "<main>{{{contents}}}</main>{{#partials.news}}<aside>{{{partials.news}}}</aside>{{/partials.news}}",
    );
    let site = site.path();
    write(
        site,
        "index.md",
        "---\ntitle: Home\ntags: [updates]\n---\n# Home\n",
    );
    write(
        site,
        "partials/news.md",
        "---\ntitle: Hidden title\ntags: [secret]\n---\n**Latest** [news](/news)\n",
    );
    write(site, "partials/footer/contact.md", "Contact us\n");
    write(site, "partials/footer.md", "Footer\n");
    write(site, "partials/title.md", "A partial named title\n");
    write(site, "partials.html.md", "A regular page\n");
    write(site, "posts/partials/story.md", "A nested regular page\n");
    write(
        site,
        "layouts/partials.html.html",
        "{{title}}:{{{partials.title}}}",
    );
    write(
        site,
        "layouts/tags/default.html",
        "{{#posts}}{{title}}:{{{partials.footer}}}{{/posts}}|{{{partials.news}}}|{{{partials.footer/contact}}}",
    );

    build(site);

    let index = read(site, "dist/index.html");
    assert!(index.contains("<main><h1>Home</h1></main>"));
    assert!(
        index.contains("<aside><p><strong>Latest</strong> <a href=\"/news\">news</a></p></aside>")
    );
    assert!(!index.contains("Hidden title"));
    assert!(!site.join("dist/partials").exists());
    assert!(!site.join("dist/tags/secret").exists());
    assert!(site.join("dist/posts/partials/story.html").is_file());
    assert_eq!(
        read(site, "dist/partials.html.html"),
        ":<p>A partial named title</p>"
    );
    let tag = read(site, "dist/tags/updates/index.html");
    assert_eq!(
        tag,
        "Home:<p>Footer</p>|<p><strong>Latest</strong> <a href=\"/news\">news</a></p>|<p>Contact us</p>"
    );
}

#[test]
fn hides_empty_and_missing_partials_and_refreshes_them_on_rebuild() {
    let site = site(
        "{{#partials.news}}<aside>{{{partials.news}}}</aside>{{/partials.news}}{{^partials.news}}No news{{/partials.news}}",
    );
    let site = site.path();

    // A site without a partials directory still builds.
    build(site);
    assert_eq!(read(site, "dist/index.html"), "No news");

    write(site, "partials/news.md", "An update");
    build(site);
    assert_eq!(
        read(site, "dist/index.html"),
        "<aside><p>An update</p></aside>"
    );

    for contents in [
        "",
        " \t\n\r\n",
        "---\ntitle: News\n---\n \n",
        "[link]: /unused\n",
    ] {
        write(site, "partials/news.md", contents);
        build(site);
        assert_eq!(read(site, "dist/index.html"), "No news");
        assert!(!site.join("dist/partials").exists());
    }

    write(site, "partials/news.md", "Another update");
    build(site);
    assert_eq!(
        read(site, "dist/index.html"),
        "<aside><p>Another update</p></aside>"
    );
    fs::remove_file(site.join("partials/news.md")).unwrap();
    build(site);
    assert_eq!(read(site, "dist/index.html"), "No news");
}

#[test]
fn applies_ignore_files_and_markdown_options_to_partials() {
    let site = site(
        "{{{partials.news}}}|{{partials.news}}|{{{partials.ignored}}}|{{{partials.drafts/news}}}",
    );
    let site = site.path();
    write(site, "partials/news.md", "<b>News</b>");
    write(site, "partials/ignored.md", "Ignored content");
    write(site, "partials/drafts/news.md", "Draft content");

    for allow_html in [false, true] {
        write(
            site,
            "vss.toml",
            &format!(
                "[build]\nignore_files = [\"ignored.md\", \"partials/drafts\"]\n[build.markdown]\nallow_dangerous_html = {allow_html}\n"
            ),
        );
        build(site);
        let index = read(site, "dist/index.html");
        if allow_html {
            assert_eq!(
                index,
                "<p><b>News</b></p>|&lt;p&gt;&lt;b&gt;News&lt;/b&gt;&lt;/p&gt;||"
            );
        } else {
            assert_eq!(
                index,
                "<p>&lt;b&gt;News&lt;/b&gt;</p>|&lt;p&gt;&amp;lt;b&amp;gt;News&amp;lt;/b&amp;gt;&lt;/p&gt;||"
            );
        }
        assert!(!site.join("dist/partials").exists());
    }
}
