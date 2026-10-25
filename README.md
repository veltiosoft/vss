# vss

Veltiosoft Static Site Generator.

## Documentation

Read: https://vss.veltiosoft.dev/

## Create a site

```sh
vss new my-site
cd my-site
vss build
vss serve
```

You can also run `vss new .` in an empty directory. The destination must be
nonexistent or completely empty, including hidden files and directories.
Existing files are never overwritten; there is no `--force` option.
Generated text files use UTF-8 and LF line endings on every OS.

Set `base_url` in `vss.toml` to your public URL, then publish the contents of
`dist/`. The development server rebuilds on changes; reload the browser manually.

## Input files

Markdown discovery skips directories whose names start with `.`, at any depth
below the site root. This applies to pages and Markdown partials, so files inside
`.claude/`, `.github/`, or `posts/.drafts/` are not rendered or included in tag
pages. The site root itself and dot-prefixed Markdown filenames are still allowed.
Static files and templates retain their existing behavior, including copying
`static/.well-known/` and loading explicitly configured dot-prefixed directories.

Symbolic links are excluded from builds, including links to Markdown, partials,
templates, static files, and directories containing them. Use regular files and
directories for content you want to publish. In `vss serve`, replacing an input
with a symbolic link triggers a full rebuild that removes its previous output.

YAML front matter is limited to 64 KiB per file. Each file can have at most
128 tags, with at most 256 UTF-8 bytes per tag. These limits also apply to
Markdown partials. Invalid front matter or exceeded limits cause a build error;
the Markdown body is not subject to the front matter size limit.

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

