# bug: インストーラースクリプトに検証と正しい配布先を追加する

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-install-script-verification
- Polished: {YYYY-MM-DD}

## 目的

`scripts/install.sh` / `install.ps1` は署名・チェックサム検証なしでリモートのアーカイブを展開し、バイナリを配置する。`install.ps1` は旧リポジトリを指し、現行リリースと配置が一致しない。配布経路の整合性を確保する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `scripts/install.sh` は `curl --fail --location` で GitHub Releases のアーカイブを取得し、`tar` / `unzip` で展開する。チェックサム・署名検証はない。
- `curl` に `--proto '=https'` / `--tlsv1.2` がなく、リダイレクトで HTTP へ落ちる余地がある。
- `if [ ext = "zip" ]` は `$ext` の typo で、Windows 分岐では `ext` が未設定のため zip を tar で展開しようとする。
- `scripts/install.ps1` の URL は `veltiosoft/go-vss`（旧リポジトリ）を指し、`vss_windows_amd64/vss.exe` という入れ子配置を期待する。現行リリースは `veltiosoft/vss` のフラットな `vss.exe`。
- macOS の bsdtar / unzip は `..` を含むアーカイブを拒否することを確認したが、環境により挙動が異なる。

## 設計方針

- `curl --proto '=https' --tlsv1.2 --fail --show-error` を使う。
- リリースに SHA-256 チェックサムを付け、ダウンロード後に検証してから `chmod` / `mv` する。可能なら署名検証も行う。
- `ext` の参照を `$ext` に修正し、Windows 分岐を zip 展開にする。
- `install.ps1` の URL を `veltiosoft/vss` に直し、現行のフラットなアーカイブ配置に合わせる。
- アーカイブ展開は必要なファイルだけを対象にし、展開先外への書き込みを拒否する。

## 完了条件

- [ ] チェックサムが一致しないダウンロードでインストールが失敗する。
- [ ] 正しいチェックサムで macOS / Linux / Windows のインストールが成功する。
- [ ] Windows 分岐が zip を正しく展開する。
- [ ] `install.ps1` が `veltiosoft/vss` の現行リリースから取得し、`vss.exe` を配置できる。
- [ ] `curl` が HTTPS のみを許可し、HTTP へのリダイレクトを拒否する。

## 関連

- [0053](0053-bug-self-update-no-signature.md): 自己更新の署名検証。
- [0055](0055-bug-release-workflow-tag-injection.md): リリース成果物の署名・attestation。
