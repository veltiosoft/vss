# bug: self update に署名検証と確認を追加する

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-self-update-signature
- Polished: {YYYY-MM-DD}

## 目的

`vss self update` は確認なしで GitHub Releases のバイナリを置き換えるが、署名検証がない。リポジトリの write 権限やトークンが侵害された場合、全ユーザーに悪性バイナリを自動適用できるため、成果物の署名検証と確認を追加する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `Cargo.toml` の `self_update` は `default-features = false` で `archive-tar`、`archive-zip`、`compression-*`、`rustls` を有効にするが、`signatures` を有効にしていない。
- `src/subcommand_self/update.rs` の `update` は `no_confirm(true)` で確認なしに更新する。
- TLS は rustls + webpki-roots で保護されるが、成果物の署名・チェックサム検証はない。
- `self_update` は tar/zip を一時ディレクトリへ展開する。tar 0.4.44 の `unpack_in` 脆弱性と組み合わせると、展開先外への書き込み・chmod の余地がある。

## 設計方針

- `self_update` の `signatures` feature を有効化し、リリース成果物を zipsign で署名する。
- `no_confirm(true)` を外し、バージョン・成果物・ハッシュを表示して確認を求める。非対話環境向けに明示的な `--yes` を用意する。
- リリースに SHA-256 チェックサムと GitHub Artifact Attestations を付け、更新時に検証する。
- tar / rustls / zip / time を修正版へ更新する。[0054](0054-update-vulnerable-dependencies.md) と連動する。

## 完了条件

- [ ] 署名のない成果物、署名が不正な成果物、ハッシュが一致しない成果物で更新が失敗する。
- [ ] 正しい署名・ハッシュの成果物で更新できる。
- [ ] 既定では確認を求め、`--yes` 指定時だけ非対話で更新する。
- [ ] リリースワークフローが署名・チェックサム・attestation を生成する。[0055](0055-bug-release-workflow-tag-injection.md) と連動する。
- [ ] 更新経路の tar/rustls が修正版であることをテストまたは `cargo audit` で確認する。

## 関連

- [0054](0054-update-vulnerable-dependencies.md): 脆弱な依存クレートの更新。
- [0055](0055-bug-release-workflow-tag-injection.md): リリース成果物の署名・attestation。
