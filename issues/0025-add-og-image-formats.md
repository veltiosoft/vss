# OGP 画像の対応形式を整理し、絵文字からの PNG 生成を検討する

## 管理情報

- 状態: 未解決
- 優先度: P3
- 元 issue: [veltiosoft/vss#25](https://github.com/veltiosoft/vss/issues/25)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: なし
- ラベル: `enhancement`, `priority:P3`
- 作成日時: 2024-04-08T02:12:23Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:35Z

## トリアージ結果

状態: [#12](0012-bug-og-image-frontmatter.md) の仕様と、画像生成の要件整理待ち。
確認対象: `main` の `663b819`（v0.21.0）。

旧 Go 版の `go/builder.go` と `go/frontmatter.go` には、絵文字を PNG に変換して `og_image` に設定する実装がある。
一方、現在の Rust 版には `og_image` の受け渡しも画像生成もないため、旧 Go 版の PNG 対応だけを根拠にクローズしない。

## 作業の切り分け

- [#12](0012-bug-og-image-frontmatter.md): 明示的に指定した `og_image` をテンプレートへ渡し、不正な画像 URL の出力を修正する。
- この issue: OGP 画像の形式を整理し、絵文字からの PNG 生成を Rust 版で提供するか決める。

## 調査と完了条件

- [ ] 当初の SVG 対応調査では、対象サービスと `og:image:type` の指定条件を明記する。
- [ ] 推奨する画像形式を決め、ドキュメントに記載する。
- [ ] 絵文字からの PNG 生成を復元するか決める。
- [ ] 復元する場合は、明示的な `og_image` を優先すること、生成先、失敗時の動作を決め、回帰テストを追加する。

現在のトリアージでは、外部サービス側の SVG 対応状況は検証していない。

<details>
<summary>起票時の内容</summary>

確認したいこと
- [ ] image:type に `image/svg+xml` を指定してもだめか？

参考
- https://ogp.me/
- [og:imageにSVGは指定できないっぽい](https://yoshinorin.net/articles/2020/12/01/ogpimage-does-not-support-svg/)

</details>
