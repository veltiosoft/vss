# Issue 一覧

今回トリアージした GitHub issue 11 件を Markdown として保存している。
未クローズの 9 件はこのディレクトリ直下、解決済みの 2 件は `closed/` に置く。
既存のタグページのソートに関する issue はそのまま残す。

## 管理方法

- 各ファイルに、状態、優先度、元の GitHub URL、ラベル、担当者、日時を記録する。
- GitHub の本文、起票時の内容、コメントを保存し、今回移行した issue 同士の参照は相対リンクにする。
- GitHub の状態と更新日時は取得時の記録であり、このディレクトリとは自動同期しない。
- 未解決の issue を対応済みにしたら、解決方法を記録し、ファイルを `closed/` へ移す。
- GitHub 側の未解決 issue は今回のファイル作成ではクローズしていない。

## P1: 実装済み、クローズ前の確認

- [#12: bug: Rust 版で frontmatter の `og_image` をテンプレートへ渡せるようにする](0012-bug-og-image-frontmatter.md)
  - 実装とローカル検証済み。未クローズ。
- [#31: bug: Rust 版の `vss new` を実装し、空の現在ディレクトリへの生成を可能にする](0031-bug-new-command.md)
  - 実装と macOS での検証済み。Windows CI の検証待ち。未クローズ。

## P2: 計画対応

- [#61: feat: AI Agent 向けの SKILL を指定ディレクトリへインストールする CLI を追加する](0061-add-agent-skill-install.md)
  - 実装済み。利用者が親ディレクトリを指定し、同梱 SKILL を `vss/SKILL.md` に配置する。

- [#20: docs: 現在の Rust 版の利用方法をドキュメントにまとめる](0020-doc-rust-usage.md)
- [#33: 使用中の vss version を vss.toml で設定できるようにする](0033-add-project-vss-version.md)
- [#50: dev server でファイル監視による Web ページの自動リロード機能を追加する](0050-add-serve-live-reload.md)

## P3: 前提作業や要件整理待ち

- [#3: feat: External CMS Support](0003-add-external-cms-support.md)
- [#6: feat: wasm plugin system](0006-add-wasm-plugin-system.md)
- [#25: OGP 画像の対応形式を整理し、絵文字からの PNG 生成を検討する](0025-add-og-image-formats.md)
- [#32: 調査: new コマンドで生成したファイルの改行が CRLF になるかも？](0032-other-new-file-line-endings.md)

## 既存の未解決 issue

- [タグページの投稿を pub_datetime でソートする](tag-page-sort-by-pub-datetime.md)

## 解決済み

- [#11: fix (serve): ensure that fiels added later are also added to the monitaring target](closed/0011-bug-serve-watch-new-files.md)
- [#37: dist を指定できるようにする](closed/0037-add-custom-dist.md)
