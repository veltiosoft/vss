# fix (serve): ensure that fiels added later are also added to the monitaring target

## 管理情報

- 状態: 解決済み
- 優先度: なし（解決済み）
- 元 issue: [veltiosoft/vss#11](https://github.com/veltiosoft/vss/issues/11)
- GitHub の状態（取得時）: CLOSED (COMPLETED)
- 起票者: @zztkm
- 担当者: @zztkm
- ラベル: なし
- 作成日時: 2023-11-23T23:05:10Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:46Z
- 完了日時: 2026-10-25T00:13:46Z

元の GitHub issue の本文は未記入。
解決の根拠は以下のコメントに記録されている。

## 解決方法

GitHub の完了コメントを以下に保存する。

### @zztkm (2026-10-25T00:13:45Z)

[元のコメント](https://github.com/veltiosoft/vss/issues/11#issuecomment-6391283794)

現在の Rust 版では、サイトのルートを `RecursiveMode::Recursive` で監視しています（`src/subcommand_serve.rs:211`）。

`main` の `663b819`（v0.21.0）をビルドし、macOS の一時サイトでサーバー起動後に次を確認しました。

- 新しく追加した Markdown がビルドされ、その後の再編集も反映される。
- 新しく追加した多階層ディレクトリ内の Markdown がビルドされ、その後の再編集も反映される。
- 新しく追加した多階層ディレクトリ内の静的ファイルがコピーされ、その後の再編集も反映される。

新規ファイルと新規ディレクトリの追加後も監視されることを確認できたため、実装済みとしてクローズします。
ブラウザーの自動リロードは別要件で、引き続き [#50](../0050-add-serve-live-reload.md) で扱います。

実ファイル監視の今回の確認は macOS のみです。
Windows と Linux での実ファイル監視は今回未検証です。
