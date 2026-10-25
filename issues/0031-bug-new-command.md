# bug: Rust 版の `vss new` を実装し、空の現在ディレクトリへの生成を可能にする

## 管理情報

- 状態: 実装済み（Windows CI の検証待ち、未クローズ）
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

- [x] 存在しないディレクトリを指定して、ビルド可能なサイトの雛形を生成できる。
- [x] 空の既存ディレクトリと、空の現在ディレクトリ `.` に生成できる。
- [x] 既存ファイルがある場合は黙って上書きせず、理由を表示して失敗する。
- [x] 上書きオプションを設けるか決め、対応するヘルプとテストを追加する。
- [x] 生成したサイトを `vss build` でビルドする回帰テストを追加する。
- [ ] 生成ファイルの改行方針を決め、[#32](0032-other-new-file-line-endings.md) の検証条件を満たす。

## 実装と検証（未クローズ）

- `src/main.rs` の未実装分岐を `subcommand_new::run` に接続し、`src/skeleton/` に埋め込み用の最小サイトを追加した。
- 存在しないディレクトリ、空の既存ディレクトリ、空の現在ディレクトリへの生成に対応した。
- 空でないディレクトリ、通常ファイル、生成先のシンボリックリンクは理由を表示して拒否する。ドットファイルや空のサブディレクトリも拒否対象に含める。
- 上書きオプションは提供しない。`create_new` により、事前検査後に作られた既存ファイルも上書きしない。
- 生成ファイルは UTF-8 と LF 改行に統一する。埋め込みテンプレートが CRLF でチェックアウトされた場合も、書き込み前に LF に変換する。
- `tests/new_command.rs` の 7 件で生成、生成サイトのビルド、既存ファイルの保持、引数、ヘルプ、生成先リンクの拒否を確認した。
- 生成したサイトを `vss serve` で起動し、トップページが HTTP 200 で返ることを確認した。
- `README.md` と `site/docs/new.md` に使用方法を追加した。
- `cargo test --all --locked` と Clippy、フォーマット検査が macOS で成功した。

### 残る検証

`.github/workflows/ci.yaml` に macOS と Windows の生成テストを追加した。
Windows では CRLF でチェックアウトし、生成物のバイト列に CR が含まれないことと、生成後のビルドを検証する。
この環境では Windows の CI は実行できていないため、[#32](0032-other-new-file-line-endings.md) の Windows 検証は未完了として残す。

クローズは行っていない。

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
