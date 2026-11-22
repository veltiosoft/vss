---
title: 全ページに共通のフッターを表示する
description: Markdown 部品を使って、問い合わせ先などのフッターの文章を一か所で管理します。
---

# 全ページに共通のフッターを表示する

問い合わせ先など、すべてのページの末尾に載せたい文章を 1 つの Markdown ファイルにまとめます。

## フッターの原稿

`vss.toml` と同じディレクトリに `partials/` を作り、`partials/footer.md` に共通の文章を書きます。
`partials/` に置いた Markdown は、単独のページにはならず、レイアウトに差し込める部品になります。

```markdown
このサイトへのお問い合わせは [contact@example.com](mailto:contact@example.com) まで。
```

## 共通レイアウトのフッター

`layouts/default.html` の `</main>` より後、`</body>` の前に、次のコードを入れます。
すでに `<footer>` がある場合は枠を追加せず、中の文章を `{{{partials.footer}}}` に置き換えます。

```html
<footer>
  {{{partials.footer}}}
</footer>
```

`partials.footer` は `partials/footer.md` を指します。
波括弧を 3 つ使うと、Markdown から変換した HTML が差し込まれ、リンクが表示されます。

この変更が反映されるのは、`layouts/default.html` を使うページです。
記事やタグページが別のレイアウトを使っている場合は、そのレイアウトにも同じ部品を差し込みます。

## 複数のページで表示を確認する

サイトルートでビルドします。

```sh
vss build
```

`dist/index.html` と、同じレイアウトを使う別のページをブラウザーで開き、両方の末尾に同じ問い合わせ先が表示されることを確認します。
次に `partials/footer.md` の問い合わせ先だけを書き換えて再ビルドし、両方の表示が更新されることを確認します。

[クックブック](./cookbook.html)
