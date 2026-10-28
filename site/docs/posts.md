---
title: 下書きと記事一覧
description: draft による公開除外と、記事一覧およびタグ一覧の作成方法
---

# 下書きと記事一覧

## 公開対象と一覧対象を指定する

Markdown の YAML front matter で、`draft` と `is_post` を指定できます。
どちらも省略時は `false` です。
文字列の `"true"` や `"false"`、`null`、数値はビルドエラーになります。

```markdown
---
title: 最初の記事
description: 記事の紹介
author: Author
pub_datetime: "2026-10-28T09:00:00+09:00"
is_post: true
draft: false
tags:
  - news
---

# 最初の記事

記事の本文です。
```

`draft: true` の Markdown は、個別ページ、記事一覧、タグ一覧、タグページの投稿から除外します。
`partials/` の Markdown に指定した場合は、その部品もテンプレートへ渡しません。
`vss serve` 中に公開から下書きへ変更すると、以前の生成ページと不要になったタグページを削除し、一覧を更新します。
下書きを出力するプレビューオプションはありません。

下書き制御は Markdown の変換と一覧生成に適用します。
静的ファイルのコピーは front matter を調べないため、非公開の原稿や添付ファイルを `static/` に置かないでください。
同じ出力先に静的 HTML がある場合は、下書きの生成ページの代わりにその静的ファイルが残ります。

`is_post: true` の公開ページが、通常ページの `posts` に入ります。
タグのない記事も対象です。
通常の固定ページや一覧ページには指定しないでください。
隠しディレクトリ内の原稿、`build.ignore_files` で除外した原稿、シンボリックリンク、Markdown 部品は記事一覧に入りません。

## 記事一覧を作る

`posts/index.md` と、対応する `layouts/posts/index.html` を作成します。
出力は通常のページと同じく `dist/posts/index.html` です。
一覧専用の出力先を自動で追加する機能ではないため、一覧ページ自身の出力先をファイル配置で決められます。

`posts/index.md`:

```markdown
---
title: 記事一覧
---

# 記事一覧
```

`layouts/posts/index.html`:

```html
<!doctype html>
<html lang="ja">
<head><meta charset="utf-8"><title>{{title}} | {{site_title}}</title></head>
<body>
  <main>
    {{{contents}}}
    <ul>
      {{#posts}}
      <li>
        <a href="{{url}}">{{title}}</a>
        {{#pub_datetime}}<time datetime="{{pub_datetime}}">{{pub_datetime}}</time>{{/pub_datetime}}
        <p>{{description}}</p>
      </li>
      {{/posts}}
    </ul>
    {{^posts}}<p>公開記事はありません。</p>{{/posts}}
  </main>
</body>
</html>
```

各記事で参照できる値は `title`、`description`、`author`、`pub_datetime`、`url`、`tags` です。
記事の `tags` はタグ名の文字列リストです。
`url` はサイトルートからのパスで、各パス要素をパーセントエンコードします。
出力ファイル名自体は変更しません。

記事は `pub_datetime` の降順です。
RFC 3339 の日時はタイムゾーンを含めて比較し、`YYYY-MM-DD` の日付は UTC の午前 0 時として比較します。
未指定や解釈できない日付は末尾に置き、同日時や日付欠落時は URL の昇順に並べます。
元の `pub_datetime` はそのままテンプレートに渡すため、不正な日付を表示したくない場合は原稿を修正してください。
他のジェネレーターから移行する際、`date` は自動変換しません。
`pub_datetime` へ移し、日付形式とタイムゾーンを確認してください。

## タグ一覧を作る

通常ページの `all_tags` には、`posts` に含まれる記事のタグを集計したリストを渡します。
各要素は `name`、`url`、`count` を持ち、タグ名の昇順に並びます。
同じ記事で同じタグを繰り返しても件数は 1 と数えます。
パストラバーサルや Windows で無効なファイル名になるタグは含めません。

たとえば `tags/index.md` を作成し、`layouts/tags/index.html` で次のように描画します。

```html
<!doctype html>
<html lang="ja">
<head><meta charset="utf-8"><title>{{title}} | {{site_title}}</title></head>
<body>
  <main>
    {{{contents}}}
    <ul>
      {{#all_tags}}
      <li><a href="{{url}}">{{name}}</a>（{{count}} 件）</li>
      {{/all_tags}}
    </ul>
    {{^all_tags}}<p>タグはありません。</p>{{/all_tags}}
  </main>
</body>
</html>
```

タグの URL は `build.tags.url_pattern` に従います。
`build.tags.enable = false` でも一覧データは使えますが、リンク先のタグページは自動生成しません。
その場合はリンク先を別途用意するか、リンクを表示しないテンプレートにしてください。

既存のタグページは互換性のため、`is_post` に関係なく公開 Markdown のタグを対象にします。
そのため、タグ付きの固定ページがあると、タグページの投稿数と `all_tags` の件数が異なる場合があります。
タグページ内の `posts` はそのタグの公開ページを日付順に並べたもので、通常ページの `posts` とは対象が異なります。

一覧ページも通常のページと同じテンプレート選択と出力先の優先順位に従います。
静的ファイルとの重複では生成ページが優先されます。
タグの URL パターンが手書きページと重なると、既存のタグ生成が上書きします。
たとえば `url_pattern = "/tags/{tag}"` とタグ名 `index.html` の組合せは、`tags/index.md` の出力先と重なるため避けてください。

## 変更時の再生成

`vss serve` は、記事の追加、削除、リネーム、公開状態、一覧対象、メタデータの変更を一覧へ反映します。
一覧データが変わった場合は、それを参照できるすべての通常ページを再描画します。
記事本文だけの変更では、変更した記事とタグページを更新します。
