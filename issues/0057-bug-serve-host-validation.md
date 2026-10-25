# bug: vss serve に Host 検証とセキュリティヘッダを追加する

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-serve-host-validation
- Polished: {YYYY-MM-DD}

## 目的

`vss serve` はローカル開発サーバーだが、Host ヘッダを検証せず、セキュリティヘッダも付かない。DNS rebinding などでローカル配信内容を読む余地があり、dotfile を配信する設定では `.env` などが露出する。Host を限定し、ヘッダを追加する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `src/subcommand_serve.rs` の `run_serve` は `SocketAddr::from(([127, 0, 0, 1], port))` でバインドし、`ServeDir` で dist を配信する。バインドはローカル限定で良好。
- `ServeDir` のパストラバーサル防御は動的確認済み。`/../outside_secret.txt`、`/..%2f...`、`/%2e%2e/...`、`/%00` はすべて 404。
- Host ヘッダ検証がなく、`X-Content-Type-Options: nosniff` などのヘッダがない。
- dist 内の dotfile をそのまま配信する。`static = "."` のサイトで `GET /.env` が 200 で `.env` の内容を返すことを確認した。[0052](0052-bug-static-layouts-outside-site.md) と関連する。
- ローカル専用・GET のみ・状態変更 API なしのため、CSRF の影響は小さい。

## 設計方針

- Host ヘッダを `localhost` / `127.0.0.1`（ポート付き）に限定するミドルウェアを追加する。
- `X-Content-Type-Options: nosniff` などのセキュリティヘッダを付ける。
- dotfile を配信しないオプションを用意する、または [0052](0052-bug-static-layouts-outside-site.md) の入力検証で根本対処する。
- 既定のバインドは `127.0.0.1` のまま維持する。

## 完了条件

- [ ] 許可外の Host ヘッダでリクエストした場合、404 または 400 を返す。
- [ ] `localhost` / `127.0.0.1` の Host で従来どおり配信できる。
- [ ] `X-Content-Type-Options: nosniff` などのヘッダが付く。
- [ ] dotfile 配信の既定動作を決め、テストで確認する。
- [ ] パストラバーサル防御の回帰テストが引き続き成功する。

## 関連

- [0052](0052-bug-static-layouts-outside-site.md): `static` / `layouts` のサイト外参照。
