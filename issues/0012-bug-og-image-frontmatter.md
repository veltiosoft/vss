# bug: Rust 版で frontmatter の `og_image` をテンプレートへ渡せるようにする

## 管理情報

- 状態: 未解決
- 優先度: P1
- 元 issue: [veltiosoft/vss#12](https://github.com/veltiosoft/vss/issues/12)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: @zztkm
- ラベル: `bug`, `priority:P1`
- 作成日時: 2023-11-23T23:12:37Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:20Z

## 現在の問題

確認対象: `main` の `663b819`（v0.21.0）。

`site/layouts/posts/default.html:11` は `{{og_image}}` を参照しているが、Rust 版の `FrontMatter` と `RenderContext` に `og_image` がない。
frontmatter に `og_image: image.png` を記載しても、テンプレートでは空文字になる。
リポジトリの `site/` をビルドすると、投稿の `og:image` は画像 URL ではなく `https://vss.veltiosoft.dev/` になる。

旧 Go 版には `og_image` の受け渡しと絵文字からの PNG 生成があるが、現在の Rust 版には引き継がれていない。

## 完了条件

- [ ] frontmatter の `og_image` を読み込み、ページのテンプレートから参照できる。
- [ ] 明示した画像が `og:image` に反映されることを回帰テストで確認する。
- [ ] 未指定時の出力を決め、サイトルートを誤って画像 URL として出力しない。
- [ ] 絶対 URL とサイト内の画像パスの扱いを定義し、URL が壊れないようにする。
- [ ] サイトの投稿テンプレートと設定例を、実装した仕様に合わせる。

## 関連 issue

- [#25](0025-add-og-image-formats.md): 画像形式の調査と、絵文字からの PNG 生成。こちらで扱う明示的な `og_image` の受け渡しとは分けて進める。

## コメント

### @zztkm (2024-03-27T15:38:46Z)

[元のコメント](https://github.com/veltiosoft/vss/issues/12#issuecomment-2023088559)

yaml frontmatter に書けるようにして、template に変数記述してもらうことで解決できるかも。

今度自ブログで試してみる
