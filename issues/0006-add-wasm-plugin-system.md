# feat: wasm plugin system

## 管理情報

- 状態: 未解決
- 優先度: P3
- 元 issue: [veltiosoft/vss#6](https://github.com/veltiosoft/vss/issues/6)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: @zztkm
- ラベル: `enhancement`, `priority:P3`
- 作成日時: 2023-11-05T06:27:31Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:38Z

## トリアージ結果

状態: 設計待ち。
確認対象: `main` の `663b819`（v0.21.0）。

現在の Rust 版に Wasm プラグインの実行基盤はない。
上記の標準入出力やデータ形式は初期案として残し、採用する方式は次の要件を決めてから選ぶ。

## 実装前に決めること

- [ ] 最初に対応する具体的なユースケース。
- [ ] プラグインを呼び出すタイミングと、変更を許すデータ。
- [ ] 入出力の形式と互換性の管理方法。
- [ ] ファイルシステムとネットワークへのアクセス権限。
- [ ] タイムアウト、リソース上限、エラー時の動作。

[#3](0003-add-external-cms-support.md) の外部 CMS 対応を、このプラグイン基盤に依存させるかは未決定。
古い提案であることだけを理由に `wontfix` にはしない。

<details>
<summary>起票時の内容</summary>

Wasm (WASI) を使ったプラグインシステムを作りたい。

プラグインシステムの流れ

1. vss 側でプラグインに渡すデータを用意
2. 標準入力を通じて、gRPC or json でデータを受け渡し
3. wasm 側で処理するし、処理結果を標準出力に書き込み
4. vss 側で標準出力の内容をキャッチしてなんかやる

## 利用例

標準では用意していない機能を付け足したいとき
- Open Graph 用の画像を生成したいとき
- Markdown to HTML 処理方法を変更したいとき

## References

- https://github.com/sqlc-dev/sqlc/blob/856779c5df61b5ab52d772fe50b1bc716351ab04/internal/ext/wasm/wasm.go
- https://github.com/orisano/sqlc-gen-ts-d1/blob/main/cmd/sqlc-gen-ts-d1/main.go#L528

</details>
