# bug: static / layouts のサイト外参照を検証し、任意ファイルの公開を防ぐ

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-source-path-validation
- Polished: {YYYY-MM-DD}

## 目的

`static` / `layouts` はビルド入力を指定するが、現在はサイト外のパスも受け付ける。悪意ある設定や誤設定で `.env`、`.git/config`、SSH 鍵などを `dist` にコピーして公開したり、`vss serve` で配信したりできるため、入力パスを検証する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `src/subcommand_build.rs` の `copy_static_files` は `absolute_path(root, static_dir)` をそのまま `source_files` で走査し、`dist` へ `fs::copy` する。サイト外でもコピーされる。
- `load_templates` も `absolute_path(root, layouts_dir)` をそのまま走査し、サイト外の `.html` をテンプレートとして読み込む。
- 一時ディレクトリで次を確認した。
  - `static = "../outside"` で、サイト外の `secret.txt` が `dist/secret.txt` にコピーされた。
  - `static = "."` で、`dist/.git/config`、`dist/secrets/.env`、`dist/vss.toml`、`dist/index.md`、`dist/layouts/default.html` が生成された。
  - 同サイトを `vss serve` で起動し、`GET /.env` が 200 で `.env` の内容を返した。`GET /vss.toml` と `GET /index.md` も 200。
  - `layouts = "../outside"` で、サイト外のテンプレート `OUTSIDE-TEMPLATE <h1>hi</h1>` が使われた。
- `static` のコピーは symlink を除外するが、通常ファイルはサイト内外を問わずコピーする。dotfile / dot ディレクトリも除外しない。

## 設計方針

- `load_config` で `static` / `layouts` を検証し、サイトルート内の相対パスに限定する。サイト外を許す場合は明示的 opt-in とする。
- `static = "."`、`static = ""`、サイトルート自身、祖先、`dist` と重なるパスを拒否する。
- 静的コピーで dotfile / dot ディレクトリを既定で除外する。`.well-known` など必要なものだけ明示的に許可する。
- `layouts` もサイト内に限定し、テンプレート探索キーがサイト外へ解決しないことを確認する。
- サイト外 `static` / `layouts` を拒否する回帰テストを追加する。

## 完了条件

- [ ] `static` / `layouts` が絶対パス、`.`、`..`、サイト外パスのとき、ビルドがコピー・テンプレート読み込み前にエラーになる。
- [ ] `static = "."` や `static = ""` が拒否される。
- [ ] 既定の `static/` で `.env`、`.git`、`.ssh` などの dotfile / dot ディレクトリが `dist` にコピーされない。`.well-known` の扱いを決めてテストする。
- [ ] サイト外 `static` / `layouts` の opt-in を使う場合の条件をテストで確認する。
- [ ] `vss serve` でも同じ検証が適用され、サイト外ファイルを配信しないことを確認する。

## 関連

- [0051](0051-bug-dist-path-deletion.md): `dist` の検証。
- [0057](0057-bug-serve-host-validation.md): `vss serve` の Host 検証と dotfile 配信。
