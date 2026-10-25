use std::{fs, path::Path, process::Command};

const POST_TEMPLATE: &str = include_str!("../site/layouts/posts/default.html");

fn write(site: &Path, path: &str, contents: &str) {
    let path = site.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn build(site: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vss"))
        .arg("build")
        .current_dir(site)
        .output()
        .unwrap()
}

#[test]
fn renders_explicit_og_images_with_url_resolution_and_html_escaping() {
    for (base_url, image, expected) in [
        (
            "https://example.com",
            "images/cover.png",
            "https://example.com/images/cover.png",
        ),
        (
            "https://example.com/",
            "/images/cover.png",
            "https://example.com/images/cover.png",
        ),
        (
            "https://example.com/blog",
            "images/cover.png",
            "https://example.com/blog/images/cover.png",
        ),
        (
            "https://example.com/blog/",
            "/images/cover.png",
            "https://example.com/images/cover.png",
        ),
        (
            "https://example.com",
            "https://cdn.example.com/cover.png?x=1&y=2",
            "https://cdn.example.com/cover.png?x=1&amp;y=2",
        ),
        (
            "https://example.com",
            "http://cdn.example.com/cover.png",
            "http://cdn.example.com/cover.png",
        ),
        (
            "https://example.com",
            "https://cdn.example.com/my cover.png",
            "https://cdn.example.com/my%20cover.png",
        ),
        ("", "images/cover.png", "/images/cover.png"),
        ("", "/images/cover.png", "/images/cover.png"),
        (
            "https://example.com",
            "images/my cover.png",
            "https://example.com/images/my%20cover.png",
        ),
    ] {
        let site = tempfile::tempdir().unwrap();
        let site = site.path();
        write(site, "vss.toml", &format!("base_url = '{base_url}'"));
        write(site, "layouts/posts/default.html", POST_TEMPLATE);
        write(site, "layouts/raw.html", "{{og_image}}");
        let markdown = format!("---\ntitle: Post\nog_image: '{image}'\n---\n# Post\n");
        write(site, "posts/post.md", &markdown);
        write(site, "raw.md", &markdown);

        let output = build(site);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let html = fs::read_to_string(site.join("dist/posts/post.html")).unwrap();
        assert!(
            html.contains(&format!(
                "<meta property=\"og:image\" content=\"{expected}\" />"
            )),
            "image: {image}; html: {html}"
        );
        assert_eq!(
            fs::read_to_string(site.join("dist/raw.html")).unwrap(),
            image.replace('&', "&amp;")
        );
    }
}

#[test]
fn omits_og_image_meta_when_the_image_is_missing_or_empty() {
    for frontmatter in ["", "og_image: ''\n"] {
        let site = tempfile::tempdir().unwrap();
        let site = site.path();
        write(site, "vss.toml", "base_url = 'https://example.com'");
        write(site, "layouts/posts/default.html", POST_TEMPLATE);
        write(
            site,
            "posts/post.md",
            &format!("---\ntitle: Post\n{frontmatter}---\n# Post\n"),
        );
        let output = build(site);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let html = fs::read_to_string(site.join("dist/posts/post.html")).unwrap();
        assert!(!html.contains("property=\"og:image\""), "{html}");
    }
}

#[test]
fn rejects_unsupported_or_malformed_og_image_urls() {
    for image in [
        "javascript:alert(1)",
        "data:image/png;base64,abc",
        "file:///tmp/image.png",
        "https://",
        "//cdn.example.com/cover.png",
        "images\\cover.png",
    ] {
        let site = tempfile::tempdir().unwrap();
        let site = site.path();
        write(site, "vss.toml", "base_url = 'https://example.com'");
        write(site, "layouts/default.html", "{{og_image_url}}");
        write(
            site,
            "index.md",
            &format!("---\nog_image: '{image}'\n---\nHome\n"),
        );
        let output = build(site);
        assert!(!output.status.success(), "accepted {image}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("og_image"));
    }
}
