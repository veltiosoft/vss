---
title: SNS 共有用の OGP 画像と説明を設定する
description: vss でサイトのトップページに OGP 画像と説明を設定する手順です。
---

# SNS 共有用の OGP 画像と説明を設定する

既存の vss サイトのトップページに、SNS でリンクを共有したときのプレビュー用の情報（OGP）を設定します。

## 公開先とサイト共通の説明

`vss.toml` の `base_url` に、サイトの公開先 URL を設定します。
`site_description` には、ページごとの説明を省略したときに使う文章を書きます。

```toml
base_url = "https://example.com/"
site_description = "日々の制作メモを公開するサイトです。"
```

## トップページの説明と画像

`index.md` の先頭にある front matter に、タイトル、説明、画像のパスを書きます。
すでに front matter がある場合は、既存の `---` の内側に設定を追加または更新します。

```yaml
---
title: 制作ノート
description: 最新の制作メモをまとめたページです。
og_image: images/og.png
---
```

画像は `static/images/og.png` に置きます。
`og_image` を `/` で始めずに書くと、`base_url` を基準に解決します。
たとえば `base_url = "https://example.com/blog/"` なら、画像の URL は `https://example.com/blog/images/og.png` になります。
別のサイトにある画像は、HTTP(S) の絶対 URL を指定できます。

## レイアウトのメタタグ

`layouts/index.html` の `<head>` に次のコードを入れます。
`layouts/index.html` がなければ、`layouts/default.html` をコピーして作ります。
同じメタタグがすでにある場合は置き換え、`<title>` やスタイルシートの設定は残します。

```html
<meta property="og:type" content="website">
<meta property="og:url" content="{{base_url}}">
<meta property="og:title" content="{{title}}">
{{#description}}
<meta name="description" content="{{description}}">
<meta property="og:description" content="{{description}}">
{{/description}}
{{^description}}
<meta name="description" content="{{site_description}}">
<meta property="og:description" content="{{site_description}}">
{{/description}}
{{#og_image_url}}
<meta property="og:image" content="{{og_image_url}}">
{{/og_image_url}}
```

`og_image_url` は、vss が `og_image` から解決した画像 URL です。
画像が未指定なら、この例では `og:image` を出力しません。
未指定時に共通画像を使うなら、`{{^og_image_url}}` と `{{/og_image_url}}` の間にも `og:image` を書き、共通画像の絶対 URL を指定します。

## 生成したメタタグと画像の確認

サイトルートでビルドします。

```sh
vss build
```

`dist/index.html` の `<head>` を開き、`og:description` にページの説明、`og:image` に `https://example.com/images/og.png` のような公開先の絶対 URL が入っていることを確認します。
公開後は、`og:image` の URL をブラウザーで直接開き、画像が表示されることを確認します。

[クックブック](./cookbook.html)
