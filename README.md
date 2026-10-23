# vss

Veltiosoft Static Site Generator.

## Documentation

Read: https://vss.veltiosoft.dev/

## Markdown partials

Place reusable Markdown in a `partials/` directory at the site root (the working
directory where you run `vss build` or `vss serve`). These files are converted to
HTML at build time and are available in page and tag-page templates. They do not
produce standalone pages or contribute posts to tag pages.

For example, put an announcement in `partials/news.md`:

```markdown
**News:** Our opening hours have changed.
```

Then include it in `layouts/default.html`:

```html
{{#partials.news}}
<aside class="news">
  <h2>News</h2>
  {{{partials.news}}}
</aside>
{{/partials.news}}
```

Use triple braces to insert the rendered HTML. Double braces (`{{partials.news}}`)
escape it. The section is omitted when the file is absent or its rendered HTML is
empty, including blank files and files containing only YAML front matter.
Use `{{^partials.news}}No news{{/partials.news}}` for an empty-state message.

- Names are relative to `partials/`, without the `.md` extension. A nested file
  such as `partials/footer/contact.md` is referenced as `{{{partials.footer/contact}}}`.
- YAML front matter is removed before Markdown conversion; its metadata is not
  exposed as page metadata.
- `build.ignore_files` and `build.markdown.allow_dangerous_html` apply to these
  files just as they do to ordinary pages.
- `vss serve` rebuilds pages when partials are added, changed, or removed.
- `partials/` is reserved at the site root. Move Markdown intended to be published
  as standalone pages to another directory. A nested directory such as
  `posts/partials/` still contains ordinary pages.

This feature provides rendered Markdown values. HTML template includes using
ramhorns' `{{>file}}` syntax are not supported.

