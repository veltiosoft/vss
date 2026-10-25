# bug: リリースワークフローのタグ名インジェクションと権限を修正する

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-release-workflow-security
- Polished: {YYYY-MM-DD}

## 目的

`release.yaml` はタグ名をシェルへ未クォートで展開し、`GITHUB_TOKEN` 付きで実行する。タグを push できる者が `$(...)` やバッククォートを含むタグ名で任意コマンドを実行できるため、値を環境変数経由にし、権限を最小化する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `.github/workflows/release.yaml` の `gh release create ${{ steps.get_version.outputs.VERSION }}` と `gh release upload ${{ needs.github-release.outputs.version }}` は値をシェルへ直接展開する。
- Git の refname は `$`、バッククォート、`(`、`)` などを許可する。`${{ ... }}` はシェル実行前に展開されるため、`$(id)` のようなタグ名で任意コマンドが実行できる。
- ワークフローに `permissions:` がなく、`GITHUB_TOKEN` はリポジトリ既定の権限で動く。
- `actions/checkout@v5`、`dtolnay/rust-toolchain@stable` は可変タグで、SHA ピン留めされていない。
- リリース成果物の署名・attestation がない。[0053](0053-bug-self-update-no-signature.md) と関連する。

## 設計方針

- タグ名は `env:` で渡し、シェルでは必ずダブルクォートする。
- `permissions: contents: write` を明示し、不要な権限を落とす。
- アクションを commit SHA でピン留めする。
- 成果物に署名・SHA-256・attestation を付ける。
- タグ名の書式を検証する。例: `v*` や semver のみ許可する。

## 完了条件

- [ ] タグ名に `$`、バッククォート、`;` などを含むリリースで、シェルコマンドが実行されない。
- [ ] ワークフローの `permissions` が `contents: write` など最小限に設定される。
- [ ] 使用するアクションが commit SHA でピン留めされる。
- [ ] リリース成果物の署名・チェックサム・attestation が生成される。
- [ ] 正常なタグでリリース作成・アップロード・公開が成功する。

## 関連

- [0053](0053-bug-self-update-no-signature.md): 自己更新の署名検証。
- [0054](0054-update-vulnerable-dependencies.md): 依存クレートの更新。
