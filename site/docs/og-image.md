---
title: OGP 画像の指定
description: frontmatter で指定した画像を og:image に出力する方法
---

# OGP 画像の指定

frontmatter の `og_image` に、使用する画像の URL またはサイト内のパスを指定します。

```yaml
---
title: 記事のタイトル
og_image: images/cover.png
---
```

サイト内の画像は `static/images/cover.png` のように配置します。
静的ファイルは `dist/images/cover.png` にコピーされるため、`og_image` に `static/` は含めません。
vss は画像の存在確認や生成、外部 URL の取得を行いません。
絵文字からの画像生成には対応していません。

## URL の解決規則

`vss.toml` に公開先の URL を指定します。

```toml
base_url = "https://example.com/blog"
```

この設定では、テンプレート変数 `og_image_url` は次の値になります。

| `og_image` | `og_image_url` |
| --- | --- |
| `images/cover.png` | `https://example.com/blog/images/cover.png` |
| `/images/cover.png` | `https://example.com/images/cover.png` |
| `https://cdn.example.com/cover.png` | `https://cdn.example.com/cover.png` |
| 未指定または空文字 | 空文字 |

相対パスは `base_url` をディレクトリとして解決します。
`base_url` の末尾の `/` の有無で結果は変わりません。
`/` で始まるパスは公開先のオリジンのルートを基準にします。
`base_url` が未指定なら、サイト内のパスは `/images/cover.png` のようなルート相対 URL になります。
公開時には絶対 URL になるよう、`base_url` に HTTP(S) の公開先 URL を指定してください。
画像パスを解決する場合、`base_url` にクエリーやフラグメントは指定できません。

HTTP(S) 以外の URL、`//` で始まる URL、バックスラッシュや制御文字を含む指定はビルドエラーになります。

## テンプレート

`og_image` は frontmatter の値をそのまま参照する変数です。
`og_image_url` は URL を解決した変数で、未指定時には空文字になります。
`base_url` をテンプレートで重ねて連結する必要はありません。

画像未指定時に `og:image` を出力しないよう、セクションで囲みます。

```html
{{#og_image_url}}
<meta property="og:image" content="{{og_image_url}}" />
{{/og_image_url}}
```

属性の値には二重波括弧を使い、HTML のエスケープを有効にします。
