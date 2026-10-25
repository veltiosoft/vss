# docs: 現在の Rust 版の利用方法をドキュメントにまとめる

## 管理情報

- 状態: 未解決
- 優先度: P2
- 元 issue: [veltiosoft/vss#20](https://github.com/veltiosoft/vss/issues/20)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: @zztkm
- ラベル: `documentation`, `priority:P2`
- 作成日時: 2024-03-27T15:33:51Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:23Z

## 現状

確認対象: `main` の `663b819`（v0.21.0）。

README、サンプルサイト、リリース記事は存在するが、`site/docs/index.md:11` の Getting Started は `wip` のまま。
利用者が Rust 版の現行仕様を参照できるドキュメントを整備する。

## 完了条件

- [ ] インストール方法と動作確認の手順を書く。
- [ ] 最小サイトの作成から `build`、`serve`、公開までの手順を書く。
- [ ] CLI の各コマンドと主要オプションを説明する。
- [ ] `vss.toml` の設定項目と既定値を説明する（`dist` を含む）。
- [ ] frontmatter の対応項目と制限を説明する。
- [ ] テンプレート変数、テンプレートの選択規則、Markdown 部品の使い方を説明する。
- [ ] タグページの設定と生成規則を説明する。
- [ ] ドットディレクトリ、シンボリックリンク、frontmatter のサイズ制限を説明する。
- [ ] 未実装の機能を利用可能なものとして案内せず、Getting Started の手順でビルドできることを確認する。

## 関連 issue

- [#31](0031-bug-new-command.md): `new` は現在未実装。解決前の導入手順では手動作成を案内する。
- [#12](0012-bug-og-image-frontmatter.md): OGP 用の `og_image` は修正後の仕様に合わせて説明する。
- [#50](0050-add-serve-live-reload.md): 現在の `serve` は自動再ビルドのみで、ブラウザーのリロードは手動。
