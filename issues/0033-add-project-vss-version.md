# 使用中の vss version を vss.toml で設定できるようにする

## 管理情報

- 状態: 未解決
- 優先度: P2
- 元 issue: [veltiosoft/vss#33](https://github.com/veltiosoft/vss/issues/33)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: なし
- ラベル: `enhancement`, `priority:P2`
- 作成日時: 2024-09-29T14:09:22Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:29Z

## 現状

確認対象: `main` の `663b819`（v0.21.0）。

Rust 版の設定構造に利用バージョンの項目がなく、不一致の警告処理もない。
試験的に `version = "0.0.0"` を設定して v0.21.0 でビルドしても、警告は出ない。

## このリポジトリで扱う完了条件

- [ ] `vss.toml` のバージョン指定のキーと書式を決める。
- [ ] 指定されたバージョンと実行中のバージョンが異なる場合に警告を出す。
- [ ] バージョン未指定の既存設定では、従来どおり利用できる。
- [ ] 一致、不一致、未指定、不正な指定の動作をテストし、設定方法を文書化する。

## Actions 側の作業

設定されたバージョンの vss を取得する Actions は、当初の案どおり `veltiosoft/vss-actions` 側の別タスクとして管理する。
本体の設定仕様が決まってから起票または既存タスクに関連付け、この issue の本体実装とは完了条件を分ける。
このトリアージでは別リポジトリの変更や新規 issue の作成は行っていない。

<details>
<summary>起票時の内容</summary>

- vss.toml に設定されているバージョンと、実行した vss のバージョンが違う場合に警告を出す
- vss.toml に指定されたバージョンの vss を DL してくれる Actions を作る (veltiosoft/vss-actions で提供する

</details>
