# vss

Veltiosoft Static Site Generator.

## Documentation

Read: https://vss.veltiosoft.dev/

## Versioning

vss uses [Calendar Versioning](https://calver.org/). Versions follow **`YYYY.M.MICRO`**;
for example, `2026.10.0` is the first release of October 2026, and the final number
counts releases within that month. Segments are unpadded, so every version is also
a valid SemVer string.

## Create a site

```sh
vss new my-site
cd my-site
vss build
vss serve
```

You can also run `vss new .` in an existing directory, including a Git
repository. Unrelated files (including hidden files and `.git/`) are preserved,
and existing directories are reused. Conflicts with skeleton paths are checked
before any skeleton files or directories are created. Existing files are never
overwritten; there is no `--force` option. Symbolic links in the destination path
or skeleton paths are rejected; unrelated links are left untouched.
Generated text files use UTF-8 and LF line endings on every OS.

Set `base_url` in `vss.toml` to your public URL, then publish the contents of
`dist/`. The development server rebuilds on changes; reload the browser manually.

`vss new` does not create or modify a site-root `.gitignore`. Instead, both
`vss build` and `vss serve` generate a `.gitignore` containing `*` and a final
newline inside the configured output directory (`dist` in `vss.toml`). It ignores
all contents, including itself, and is recreated with the output directory on
each full build. Already tracked files remain tracked; use `git rm --cached`
if you need to remove them from Git's index.

## Install an AI Agent skill

```sh
vss skill install --dir ./path/to/skills
```

`--dir` is required and names the **parent directory**: the bundled skill is
installed at `./path/to/skills/vss/SKILL.md`. Missing parent directories are
created. Relative paths are resolved from the current working directory;
absolute paths and quoted paths containing spaces are also supported.

An existing `vss` entry (even an empty directory, file, or symbolic link) causes
an error; nothing is overwritten. Other skills in the parent directory are left
unchanged. Success prints the absolute path of the installed file. Failures
report the cause and return a nonzero exit status; newly created directories or
partial files may remain after an I/O failure.

This command works outside a site without `vss.toml`. It does not select an
Agent-specific destination, edit Agent configuration, or download external
skills. See `vss skill install --help` and the
[installation guide](https://vss.veltiosoft.dev/docs/skill.html).

## Input files

Markdown discovery skips directories whose names start with `.`, at any depth
below the site root. This applies to pages and Markdown partials, so files inside
`.claude/`, `.github/`, or `posts/.drafts/` are not rendered or included in tag
pages. The site root itself and dot-prefixed Markdown filenames are still allowed.
Static copying excludes dotfiles and dot-directories, except for `.well-known/`
at the static root. Hidden paths inside `.well-known/` are still excluded.
Templates can use internal dot-directories and explicitly configured dot-prefixed
source roots.

`dist`, `static`, and `layouts` must be relative directories inside the site.
The site root, parent traversal, absolute paths, and overlapping input/output
paths are rejected before old outputs are removed. Output paths cannot traverse
symbolic links or overlap `partials/`, `.git/`, or the configuration file.

Symbolic links are excluded from builds, including links to Markdown, partials,
templates, static files, and directories containing them. Use regular files and
directories for content you want to publish. In `vss serve`, replacing an input
with a symbolic link triggers a full rebuild that removes its previous output.

YAML front matter is limited to 64 KiB per file. Each file can have at most
128 tags, with at most 256 UTF-8 bytes per tag. These limits also apply to
Markdown partials. Invalid front matter or exceeded limits cause a build error;
the Markdown body is not subject to the front matter size limit.

## Drafts and article indexes

Set `draft: true` in Markdown front matter to exclude a page from HTML generation,
tag pages, and indexes. It also hides Markdown partials. `vss serve` removes stale
generated outputs when a published page becomes a draft. No draft-preview option
is provided. Static files are copied independently; do not put private content in
`static/`.

Set `is_post: true` to include a published page in the `posts` list available to
ordinary page templates. Both flags default to `false` and require YAML booleans;
quoted strings, numbers, and `null` are errors. Tagless articles are included.
`all_tags` provides the selected articles' tag names, URLs, and article counts.

```html
<ul>{{#posts}}<li><a href="{{url}}">{{title}}</a> {{pub_datetime}}</li>{{/posts}}</ul>
<ul>{{#all_tags}}<li><a href="{{url}}">{{name}}</a> ({{count}})</li>{{/all_tags}}</ul>
```

Create ordinary Markdown pages and matching templates for these indexes, for
example `posts/index.md` and `layouts/posts/index.html`. No additional output
paths are generated automatically. Articles are sorted by `pub_datetime`
(RFC 3339 or `YYYY-MM-DD`) descending, then URL ascending; missing or invalid
dates sort last. Date-only values use UTC midnight. The `date` field used by
other generators is not automatically mapped to `pub_datetime`.

Existing tag pages retain tagged ordinary pages regardless of `is_post`, and use
the same date ordering. See the [guide](https://vss.veltiosoft.dev/docs/posts.html)
for template fields, tag counts, and rebuild behavior.

## Portable tag names

Tags with filenames that are unsafe on Windows are excluded from rendered tag
links and tag-page generation on every OS; they do not fail the build. No filename
substitution is performed, so sanitization cannot introduce name collisions.
Excluded names contain `?`, `*`, `<`, `>`, `|`, or `"`, end with a dot or space, or
use a reserved device name such as `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, or
`LPT1`–`LPT9` (case-insensitive, including extensions). `CONIN$`, `CONOUT$`, and
the Windows-reserved superscript-digit variants are also excluded. Existing
restrictions on traversal, path separators, colons, and control characters remain.

Safe tags keep their existing URL encoding and output paths: `C++` uses
`/tags/C%2B%2B/` and `dist/tags/C++/index.html`; `a b` uses `/tags/a%20b/` and
`dist/tags/a b/index.html`. Japanese tag names are unchanged.

## Development server

The development server accepts only `localhost` and `127.0.0.1` Host headers
(with an optional port). It does not serve hidden paths, except public files
under the root `.well-known/` directory.

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

