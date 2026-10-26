# bug: dist 設定のパス検証を追加し、任意ディレクトリの削除を防ぐ

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-dist-path-validation
- Polished: {YYYY-MM-DD}

## 目的

`vss.toml` の `dist` はビルド出力先を指定するが、現在は任意のパスを受け付け、存在すれば `fs::remove_dir_all` で削除する。悪意ある設定や単純な誤設定で、サイト本体・親ディレクトリ・Git 履歴・ホームディレクトリなどを破壊できるため、パスを検証して削除対象を限定する。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `src/subcommand_build.rs` の `Config` は `dist: String` をそのまま保持し、`load_config` は `build.tags.url_pattern` のみ検証する。
- `run_build` は `load_config` の直後に `create_dist_dir(&config.dist)` を呼ぶ。
- `create_dist_dir` は `Path::new(dist_path)` が存在すれば `fs::remove_dir_all(path)` を実行してから `fs::create_dir_all` する。
- 一時ディレクトリで次を確認した。
  - `dist = "../victim"` で、サイト外の `victim/important.txt` が削除され、`victim/index.html` が生成された。
  - `dist = "."` でサイト直下の中身が削除された。最終 `rmdir` は EINVAL で失敗するが、中身の削除は完了する。
  - `dist = ".."` で親ディレクトリの中身が削除された。同じく最終 `rmdir` の失敗後も中身は消える。
  - 絶対パスの `dist` でも指定ディレクトリの中身が削除された。
- 最終要素が symlink の `dist` は symlink 自体のみ削除され、リンク先は残る。ただし親ディレクトリが symlink の場合はリンク先を辿る。
- `vss serve` も起動時に `run_build` を呼ぶため、同じ削除が起こる。

## 設計方針

- `load_config` で `dist` を検証する。空でない相対パスに限定し、絶対パス、ドライブレター、UNC、`.`、`..` を拒否する。
- `absolute_path` で正規化した `dist` がサイトルート内にあり、サイトルート自身・祖先・`static` / `layouts` / `partials` / `.git` などと重ならないことを確認する。
- サイト外の `dist` を残す必要がある場合は、明示的な opt-in（例: `dist_outside_site = true`）とし、削除前に専用マーカー（例: `.vss-dist`）の存在を要求する。または一時ディレクトリへ出力して原子的に rename し、任意パスの `remove_dir_all` を避ける。
- `dist` の symlink 祖先を `has_symlink_component` で検査する。
- 既存の `rejects_tag_patterns_outside_the_site_before_deleting_outputs` と同様に、削除前に検証し、拒否時に既存出力を変更しないテストを追加する。

## 完了条件

- [ ] `dist` が絶対パス、`.`、`..`、UNC、ドライブレター、サイト外パスのとき、ビルドが削除前にエラーになる。
- [ ] `dist` が `static` / `layouts` / `partials` / サイトルート自身と重なるとき、ビルドが削除前にエラーになる。
- [ ] `dist` がサイト内の専用ディレクトリ（既定 `dist`、`public` など）のとき、従来どおり削除・再作成してビルドできる。
- [ ] サイト外 `dist` の opt-in を使う場合の削除条件とマーカー検査をテストで確認する。
- [ ] `vss serve` でも同じ検証が適用され、起動時にサイト外ディレクトリを削除しないことを確認する。

## 対応状況

- 一時サイトを使った回帰テストで、サイト外の `dist` とシンボリックリンクの出力先が現行コードに受け付けられることを確認した。
- `load_config` で相対パス・入力との重複・Git ディレクトリ・設定ファイルの包含・シンボリックリンク祖先を検証し、削除前に拒否するようにした。
- サイト外出力の opt-in は採用しない。既定の `dist`、`public`、入れ子の専用出力先は削除・再生成できる。
- `cargo test --test config_paths` の 5 テストと、`cargo test` の既存テストを含む全テストが成功した。`build` と `serve` の不正設定拒否・既存ファイル保持を確認した。
- 未クローズ。`Completed:` は更新していない。

- PR 作成前のレビューで、大小文字を区別しない FS 上の Unicode 別名（`Ässets` / `ässets`）で入力が削除されることを再現した。`same-file` による実体・祖先の比較を追加し、回帰テストの失敗から成功を確認した。

## 関連

- [0052](0052-bug-static-layouts-outside-site.md): `static` / `layouts` のサイト外参照の検証。
