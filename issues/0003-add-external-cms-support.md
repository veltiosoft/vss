# feat: External CMS Support

## 管理情報

- 状態: 未解決
- 優先度: P3
- 元 issue: [veltiosoft/vss#3](https://github.com/veltiosoft/vss/issues/3)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: @zztkm
- ラベル: `enhancement`, `priority:P3`
- 作成日時: 2023-10-29T04:25:22Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:41Z

## トリアージ結果

状態: 要件整理待ち。
確認対象: `main` の `663b819`（v0.21.0）。

現在の Rust 版に microCMS からデータを取得する機能はない。
最初に外部 CMS のデータをどの形式でビルドへ渡すかを決める。

## 実装前に決めること

- [ ] 外部スクリプトで Markdown を生成する方式と、本体が API を呼び出す方式のどちらを対象にするか。
- [ ] microCMS のコンテンツからページやテンプレートへ渡すデータの対応関係。
- [ ] 認証情報の渡し方と、生成物に認証情報を含めない方法。
- [ ] 取得タイミング、ページネーション、取得失敗時の動作。
- [ ] 最小の動作例と、完了を確認できるテスト条件。

[#6](0006-add-wasm-plugin-system.md) の Wasm プラグインを前提にするかは未決定で、現時点では依存タスクにしない。

<details>
<summary>起票時の内容</summary>

Enable to get data from an external CMS such as [microCMS ](https://microcms.io/)to build a web site

**target**

- [ ] microCMS

</details>
