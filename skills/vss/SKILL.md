---
name: vss
description: vss で Markdown の静的サイトを作成、編集、ビルドするときに使う。vss.toml、YAML front matter、Mustache テンプレート、静的ファイル、タグページ、Markdown 部品の設定と検証を扱う。
---

# vss のサイト制作

vss は Markdown を HTML に変換する静的サイトジェネレーターです。
既存サイトを編集するときは、まず `vss.toml` と `layouts/`、対象の Markdown を読み、既存の構成に合わせてください。
インストール済みバイナリの引数は `vss --help` と各サブコマンドの `--help` で確認できます。

## サイトの作成と検証

```sh
vss new my-site
cd my-site
vss build
vss serve
```

- `vss new <DIR>` は、未作成または既存のディレクトリに雛形を生成します。
  `.git/` や隠しファイルなど、雛形と無関係な既存ファイルは保持し、必要な既存ディレクトリは再利用します。
  雛形の配置先に衝突があれば、生成開始前にエラーにし、既存ファイルは上書きしません。
  生成先までのパスや雛形の配置に使うパスにあるシンボリックリンクは拒否しますが、無関係なリンクは変更しません。
  カレントディレクトリには `vss new .` を使えます。
  サイトルートの `.gitignore` は生成も変更もしません。
- `vss build` と `vss serve` は、サイトルートをカレントディレクトリとして実行します。
  設定ファイルの既定値は `vss.toml` で、`--config ./custom.toml` で変更できます。
  `--config` はサイトルートを変更しません。
- `vss build` は出力先を削除して再生成します。
  出力ディレクトリには手書きのファイルや保存が必要なデータを置かないでください。
  `build` と `serve` は、設定の `dist` で指定した出力先の直下に、`*` と末尾改行を内容とする `.gitignore` を生成します。
  出力先を再生成するたびに同じ内容で作り直し、`.gitignore` 自身を含む生成物を Git の追跡対象から除外します。
  タグページの出力先がこの `.gitignore` と重なる設定はエラーになります。
  すでに Git で追跡されている生成物の追跡は解除しません。
- `vss serve` は初回ビルド後、変更を検知して再ビルドします。
  既定の URL は `http://localhost:8080` で、`--port 3000` のように変更できます。
  ブラウザーのリロードは手動です。
- 編集後はサイトルートで `vss build` を実行し、生成された HTML と静的ファイルを確認してください。
  公開するのは出力ディレクトリの中身です。

## vss.toml

```toml
site_title = "My site"
site_description = "Site description"
base_url = "https://example.com"
dist = "dist"
static = "static"
layouts = "layouts"

[build]
ignore_files = ["README.md"]

[build.markdown]
allow_dangerous_html = false

[build.tags]
enable = true
template = "tags/default.html"
url_pattern = "/tags/{tag}/"
```

`dist`、`static`、`layouts` はサイト内の相対ディレクトリを指定します。
サイトルートそのもの、絶対パス、`..`、入力と出力が重なる指定は使用できません。
`dist` はシンボリックリンクを経由できず、`partials/` や `.git/`、設定ファイルとも重ねられません。
公開前に `base_url` を HTTP(S) の公開先 URL に設定してください。

Markdown はサイトルートから再帰的に探索され、`posts/hello.md` は `dist/posts/hello.html` になります。
`build.ignore_files` はパスに含まれる文字列で除外します。glob パターンではありません。
`.` 始まりのディレクトリ内の Markdown は探索しませんが、`.draft.md` のようなファイル名だけでは除外されません。
公開したくない Markdown は `draft: true` を指定するか、サイト外、隠しディレクトリ、除外設定を使ってください。
下書き指定は静的ファイルのコピーには適用しないため、非公開の原稿や添付ファイルを `static/` に置かないでください。
入力のシンボリックリンクはビルド対象外です。

`static/` の中身は出力先へコピーされます。
CSS、画像、JavaScript などを配置してください。
隠しパスはコピーされませんが、静的ルート直下の `.well-known/` は例外です。
その配下の隠しパスは引き続き除外されます。

## Markdown と YAML front matter

```markdown
---
title: 記事のタイトル
description: 記事の説明
author: Author
pub_datetime: "2026-10-28T09:00:00+09:00"
post_slug: hello
is_post: true
draft: false
og_image: /images/cover.png
tags:
  - news
---

# 記事の本文
```

front matter は省略できます。
キーは `snake_case` を使います。
`draft` と `is_post` は省略時に `false` で、引用符のない真偽値を指定します。
文字列、数値、`null` はビルドエラーです。
`draft: true` は個別ページ、記事一覧、タグ一覧、タグページの投稿から除外し、Markdown 部品にも適用します。
`serve` 中に公開から下書きに変えると以前の生成物と一覧を更新します。
下書きのプレビューオプションはありません。
出力パスは Markdown のファイルパスで決まり、`post_slug` は出力先を変更しません。
`og_image` は HTTP(S) の絶対 URL またはサイト内のパスを指定し、テンプレートでは解決済みの `og_image_url` を使います。
front matter は 64 KiB、タグは 1 ファイル 128 個、1 個 256 UTF-8 バイトまでです。
パストラバーサルや Windows で無効なファイル名になるタグは、タグリンクとタグページから除外されます。

## Mustache テンプレート

`layouts/default.html` を基本テンプレートとして用意します。
本文の HTML は `{{{contents}}}` で差し込みます。
二重波括弧は HTML をエスケープするため、通常のテキスト値に使います。

```html
<title>{{title}} | {{site_title}}</title>
{{#og_image_url}}<meta property="og:image" content="{{og_image_url}}">{{/og_image_url}}
<main>{{{contents}}}</main>
{{#has_tags}}
<nav>{{#tags}}<a href="{{url}}">{{name}}</a> {{/tags}}</nav>
{{/has_tags}}
```

ページでは `site_title`、`site_description`、`base_url` と front matter の各値を参照できます。
`tags` は `name` と `url` を持つリスト、`has_tags` は表示可能なタグの有無です。
タグページのテンプレートは既定で `layouts/tags/default.html` です。
タグページでは `tag_name` と `posts` を参照し、各投稿の `title`、`description`、`author`、`pub_datetime`、`url`、`tags` を使えます。

### 記事一覧とタグ一覧

通常ページでは `posts` と `all_tags` を参照できます。
`posts` は `is_post: true` の公開ページで、タグなし記事も含みます。
固定ページや一覧ページは `is_post` を省略してください。
各記事の値は `title`、`description`、`author`、`pub_datetime`、`url`、`tags`（タグ名の文字列リスト）です。
`url` はサイトルートからのパーセントエンコード済みパスです。
`pub_datetime` の降順で、RFC 3339 の日時と `YYYY-MM-DD`（UTC 午前 0 時）を比較します。
日付欠落や不正な形式は末尾、同日時は URL 昇順です。
移行元の `date` は自動変換しないため、`pub_datetime` へ移してください。

`all_tags` は一覧対象の記事だけを集計した `name`、`url`、`count` のリストで、タグ名昇順です。
同一記事の重複タグは 1 件と数え、安全でないタグ名は除外します。
既存のタグページは互換性のため `is_post` に関係なくタグ付き公開ページを対象とし、その `posts` は通常ページとは対象が異なります。
`build.tags.enable = false` でも一覧データは渡しますが、リンク先は自動生成しません。

一覧は通常の Markdown ページと対応するテンプレートで作ります。
たとえば `posts/index.md` と `layouts/posts/index.html` を用意します。
新しい一覧用の出力先を自動生成する機能ではありません。

```html
<ul>{{#posts}}<li><a href="{{url}}">{{title}}</a> {{pub_datetime}}</li>{{/posts}}</ul>
{{^posts}}<p>公開記事はありません。</p>{{/posts}}
<ul>{{#all_tags}}<li><a href="{{url}}">{{name}}</a>（{{count}} 件）</li>{{/all_tags}}</ul>
```

一覧データの変更時はすべての通常ページを再描画し、本文だけの変更時は変更した記事を更新します。
タグの URL パターンが一覧ページの出力先と重ならないようにしてください。

再利用する Markdown はサイトルートの `partials/` に置きます。
`partials/news.md` の変換済み HTML は `{{{partials.news}}}` で参照できます。
部品は単独のページやタグページの投稿にならず、front matter の値も公開しません。
ramhorns の `{{>file}}` による HTML テンプレートの include は未対応です。

`allow_dangerous_html` は既定の `false` を維持してください。
信頼できない Markdown に対して有効化しないでください。

## この SKILL の配置

```sh
vss skill install --dir ./path/to/skills
```

指定した親ディレクトリの `vss/SKILL.md` に、バイナリ同梱の SKILL を配置します。
親ディレクトリは必要に応じて作成します。
既存の `vss/` は上書きせずエラーになります。
このコマンドだけはサイト外でも実行でき、`vss.toml` は不要です。
Agent 固有の保存先の選択、設定ファイルの変更、外部 SKILL のダウンロードは行いません。
