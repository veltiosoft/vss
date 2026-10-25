# bug: self update の未対応プラットフォームで panic を返さない

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-self-update-platform-error
- Polished: {YYYY-MM-DD}

## 目的

`vss self update` は未対応の OS / arch で `panic!` する。ユーザーに穏当なエラーを返し、プロセスを異常終了させない。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `src/subcommand_self/update.rs` の `get_target` は `("linux", "x86_64")` などの既知の組み合わせ以外で `panic!("Unsupported platform: ...")` する。
- Linux armv7、FreeBSD などで `vss self update` を実行すると panic する。
- `get_target` は `&'static str` を返すため、呼び出し側でエラーを扱えない。

## 設計方針

- `get_target` を `Result<&'static str>` に変更し、未対応プラットフォームでは `anyhow::bail!` でエラーを返す。
- `update` の呼び出し側でエラーを表示し、`std::process::exit(1)` で終了する。
- 既存の対応プラットフォームのマッピングは維持する。
- 単体テストで既知の組み合わせと未知の組み合わせを確認する。OS / arch の定数はコンパイル時固定のため、関数を引数化するなどの設計を検討する。

## 完了条件

- [ ] 未対応の OS / arch で `vss self update` が panic せず、エラーメッセージを表示して終了コード 1 を返す。
- [ ] 既知のプラットフォームで従来どおり更新を試行できる。
- [ ] `cargo test --all --locked` と Clippy が成功する。
