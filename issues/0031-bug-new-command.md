# bug: Rust 版の `vss new` を実装し、空の現在ディレクトリへの生成を可能にする

## 管理情報

- 状態: 未解決
- 優先度: P1
- 元 issue: [veltiosoft/vss#31](https://github.com/veltiosoft/vss/issues/31)
- GitHub の状態（取得時）: OPEN
- 起票者: @zztkm
- 担当者: なし
- ラベル: `bug`, `priority:P1`
- 作成日時: 2024-09-24T13:49:07Z
- GitHub 更新日時（取得時）: 2026-10-25T00:13:17Z

## 現在の問題

確認対象: `main` の `663b819`（v0.21.0）。

Rust 版では `src/main.rs:33` が `todo!()` で、`src/subcommand_new.rs` は未実装。
`vss new .` と `vss new fresh-site` は、どちらも `not yet implemented` により終了コード 101 で失敗する。
起票時の「既存ディレクトリが拒否される」問題に加えて、現在は `new` コマンド全体の実装が必要。

## 完了条件

- [ ] 存在しないディレクトリを指定して、ビルド可能なサイトの雛形を生成できる。
- [ ] 空の既存ディレクトリと、空の現在ディレクトリ `.` に生成できる。
- [ ] 既存ファイルがある場合は黙って上書きせず、理由を表示して失敗する。
- [ ] 上書きオプションを設けるか決め、対応するヘルプとテストを追加する。
- [ ] 生成したサイトを `vss build` でビルドする回帰テストを追加する。
- [ ] 生成ファイルの改行方針を決め、[#32](0032-other-new-file-line-endings.md) の検証条件を満たす。

## 関連 issue

- [#32](0032-other-new-file-line-endings.md): 生成ファイルの改行調査。この issue の実装が先行する。

<details>
<summary>起票時の内容</summary>

**vss version**

Please paste the results of running vss self version
```
vss
```

**Describe the bug**

以下のログからわかる通り、現在ディレクトリを指定すると、ディレクトリが存在していて作成できないというエラーになってしまう。

なにかファイルがあれば上書きしないようにエラーにすると良いが（その場合は上書きフラグを案内するようにしたい、なお上書きフラグはこの issue を書いた時点では未実装）、何もファイルがない場合は new コマンドをエラーにしないようにしたい。

```bash
$ vss new .
2024/09/24 22:46:22 [INFO] generate skeleton started
2024/09/24 22:46:22 [ERROR] dist directory already exists
exit status 1
```

</details>
