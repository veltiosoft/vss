# dist を指定できるようにする

## 管理情報

- 状態: 解決済み
- 優先度: なし（解決済み）
- 元 issue: [veltiosoft/vss#37](https://github.com/veltiosoft/vss/issues/37)
- GitHub の状態（取得時）: CLOSED (COMPLETED)
- 起票者: @zztkm
- 担当者: なし
- ラベル: なし
- 作成日時: 2025-02-12T12:32:00Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:44Z
- 完了日時: 2026-10-25T00:13:44Z

元の GitHub issue の本文は未記入。
解決の根拠は以下のコメントに記録されている。

## 解決方法

GitHub の完了コメントを以下に保存する。

### @zztkm (2026-10-25T00:13:43Z)

[元のコメント](https://github.com/veltiosoft/vss/issues/37#issuecomment-6391283472)

現在の Rust 版では、`vss.toml` のトップレベルで出力先を指定できます。

```toml
dist = "public"
```

`main` の `663b819`（v0.21.0）をビルドし、macOS の一時サイトで次を確認しました。

- `vss build` が `public/index.html` を生成する。
- 既定の `dist/` は生成されない。
- `vss serve` が指定した `public/` から配信する。

根拠: `src/subcommand_build.rs` の `Config.dist` と `run_build`、`src/subcommand_serve.rs` の配信先設定。
既存テストの `reloads_configured_source_and_output_directories` も成功しています。

設定ファイルでの出力先指定は実装済みとしてクローズします。
CLI の `--dist` オプションは未実装なので、必要であれば別要件として管理します。
設定方法の文書化は [#20](../0020-doc-rust-usage.md) で扱います。
