---
title: お知らせがあるときだけバナーを表示する
description: Markdown 部品にお知らせを書き、原稿が空のときはバナー全体を非表示にします。
---

# お知らせがあるときだけバナーを表示する

臨時休業やメンテナンスのお知らせを、原稿がある間だけページの上部に表示します。
掲載が終わったら原稿を空にするだけで、レイアウトを編集せずにバナーを取り下げられます。

## お知らせの原稿

`vss.toml` と同じディレクトリに `partials/` を作り、`partials/notice.md` に案内を書きます。

```markdown
棚卸しのため、**12 月 8 日**は臨時休業します。
```

## 原稿があるときだけ表示するレイアウト

`layouts/default.html` の `<body>` 内で、`<main>` の直前に次のコードを入れます。

```html
{{#partials.notice}}
<aside aria-labelledby="site-notice-label">
  <p id="site-notice-label"><strong>お知らせ</strong></p>
  {{{partials.notice}}}
</aside>
{{/partials.notice}}
```

`{{{partials.notice}}}` は `partials/notice.md` から変換した HTML を差し込みます。
その外側を `{{#partials.notice}}` と `{{/partials.notice}}` で囲むことで、ファイルがない場合や変換後の HTML が空の場合は、「お知らせ」のラベルも含めて `<aside>` 全体を出力しません。

## 表示と取り下げを確認する

サイトルートでビルドします。

```sh
vss build
```

`dist/index.html` を開き、本文の上に「お知らせ」と休業日の案内が表示されることを確認します。
次に `partials/notice.md` の中身を空にするか、ファイルを削除して再ビルドします。
ページを再読み込みして、案内だけでなく「お知らせ」のラベルも消えたことを確認します。

[クックブック](./cookbook.html)
