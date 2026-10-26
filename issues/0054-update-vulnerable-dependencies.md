# update: 更新経路の脆弱な依存クレートを修正版へ更新する

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/update-vulnerable-dependencies
- Polished: {YYYY-MM-DD}

## 目的

`Cargo.lock` の依存に、自己更新の展開・TLS・YAML 解析で到達する脆弱性と未メンテナンスのクレートがある。修正版へ更新し、CI で依存脆弱性を継続検査する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

`Cargo.lock` を OSV/RustSec に問い合わせた結果（2026-10-25）。

| crate | 版 | advisory | 修正版 | 到達性 |
|---|---|---|---|---|
| tar | 0.4.44 | RUSTSEC-2026-0067 / 0068 ほか | 0.4.46 | `self update` の tar 展開で到達 |
| rustls | 0.23.36 | RUSTSEC-2026-0285 / 0346 / 0348 / 0349 | 0.23.45 / 0.23.46 | `self update` の TLS で到達 |
| zip | 2.4.2 | RUSTSEC-2026-0367 / 0368 / 0369 | 9.0.0 | Windows 更新。`extract` 未使用のためほぼ非到達 |
| time | 0.3.46 | RUSTSEC-2026-0009 | 0.3.47 | zip 経由。低 |
| quick-xml | 0.23.1 | RUSTSEC-2026-0194 / 0195 | 0.41.0 | S3 バックエンド専用で非到達 |
| unsafe-libyaml | 0.2.11 | RUSTSEC-2024-0449 | なし | frontmatter YAML で到達 |
| serde_yaml | 0.9.34+deprecated | 非推奨・未メンテ | 置換 | frontmatter YAML で到達 |
| anyhow | 1.0.100 | RUSTSEC-2026-0190 | 1.0.103 | `downcast_mut` 未使用 |
| bytes | 1.11.0 | RUSTSEC-2026-0007 | 1.11.1 | axum/hyper 経由。低 |
| tokio | 1.49.0 | RUSTSEC-2026-0386 | 1.53.3 | `AsyncFd` 未使用 |

`quinn-proto` と `rand` は `Cargo.lock` にあるがビルドツリーに含まれない。`cargo tree -i` で確認した。

## 設計方針

- `self_update` を更新するか、`tar >= 0.4.46`、`rustls >= 0.23.46`、`zip >= 9.0.0`、`time >= 0.3.47` に固定する。
- `serde_yaml` / `unsafe-libyaml` を safe Rust の YAML パーサ（例: `serde-saphyr` / `saphyr`）へ移行する。
- `anyhow >= 1.0.103`、`bytes >= 1.11.1`、`tokio >= 1.53.3` へ更新する。
- CI に `cargo audit` または `cargo deny` を追加し、既知の脆弱性を継続検査する。

## 完了条件

- [ ] `cargo audit`（または `cargo deny`）で、到達する既知脆弱性が 0 件になる。
- [ ] `self update` の tar 展開と TLS が修正版のクレートを使う。
- [ ] frontmatter の YAML 解析が未メンテナンスの `unsafe-libyaml` に依存しない。
- [ ] `cargo test --all --locked` と Clippy、フォーマット検査が成功する。
- [ ] CI に依存脆弱性検査のステップが追加され、失敗時にビルドが止まる。

## 対応状況

- `cargo tree` と OSV API の照会で、指摘の版と advisory が現状にも該当することを確認した。追加した依存バージョン・YAML 依存の回帰テストが修正前に失敗することも確認した。
- `self_update` を 1.3.2、tar を 0.4.46、rustls を 0.23.46、zip を 9.1.1、time を 0.3.55、anyhow を 1.0.104、bytes を 1.12.1、tokio を 1.54.1 へ更新した。quick-xml は依存から除外された。
- TLS は `ureq` と `rustls` / `webpki-roots` を使い、従来のルート証明書の不信任に関する回帰テストも維持した。
- YAML を `serde-saphyr` 2.0.0 へ移行し、`serde_yaml` / `unsafe-libyaml` を除去した。既存の frontmatter 境界値・参照展開のテストは成功している。
- OSV API で更新後の lockfile 全 315 パッケージを再検査した。残った advisory は ramhorns のマクロ経由の `proc-macro-error` に対する未メンテナンス警告 RUSTSEC-2024-0370 のみ。既知脆弱性の検出はない。
- `cargo test --all --locked` の全 60 テスト、Clippy、フォーマット・差分検査が成功した。CI には失敗を無視しない `cargo audit` ジョブを追加した。
- ローカルに `cargo-audit` がないため、そのコマンド自体と GitHub 上の CI 実行は未検証。署名検証の問題は解消していない。
- 未クローズ。`Completed:` は更新していない。

## 関連

- [0053](0053-bug-self-update-no-signature.md): 自己更新の署名検証。
