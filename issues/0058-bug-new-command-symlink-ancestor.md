# bug: new コマンドの生成先検査に symlink 祖先の TOCTOU 対策を追加する

- Created: 2026-10-26
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-new-symlink-ancestor
- Polished: {YYYY-MM-DD}

## 目的

`vss new` は生成先の最終要素の symlink と空性を検査するが、検査後に祖先ディレクトリが symlink へ置き換わる競合に弱い。生成先外への書き込みを防ぐため、祖先の symlink 検査または安全なファイル生成を行う。

## 現状

確認対象: `main` の `db75abf`（v0.21.0）。

- `src/subcommand_new.rs` の `generate_site` は `fs::symlink_metadata(&destination)` で最終要素の symlink を拒否し、`fs::read_dir` で空性を確認する。
- ファイル生成は `OpenOptions::new().write(true).create_new(true)` で既存ファイルを上書きしない。
- ただし `fs::create_dir_all(parent)` は祖先の symlink を辿る。検査後に祖先が symlink へ置き換わる競合の余地がある。
- 既存テスト `refuses_symlink_destinations_including_broken_links` は最終要素の symlink を確認するが、祖先の symlink は確認していない。

## 設計方針

- 生成先の祖先を `symlink_metadata` で検査し、symlink を含む場合は拒否する。
- 可能なら `openat` / `O_NOFOLLOW` 相当で生成し、検査と生成の間の競合を避ける。
- 最終要素の symlink 検査と `create_new` は維持する。
- 祖先 symlink、破損 symlink、空でないディレクトリの回帰テストを追加する。

## 完了条件

- [ ] 祖先に symlink を含む生成先で `vss new` が失敗し、リンク先にファイルを生成しない。
- [ ] 最終要素の symlink、破損 symlink、空でないディレクトリの既存動作が維持される。
- [ ] 通常のネストしたディレクトリと空の現在ディレクトリで生成・ビルドが成功する。
- [ ] 競合を再現するテスト、または `O_NOFOLLOW` 相当の実装で検査と生成の間の隙間がないことを確認する。

## 対応状況

- 祖先がシンボリックリンクの `linked/site` で現行コマンドが成功し、リンク先へ生成することを回帰テストで確認した。
- `cap-std` / `cap-fs-ext` のディレクトリハンドルと `open_dir_nofollow` を使い、生成先・雛形のサブディレクトリを一要素ずつ開くようにした。ファイルも保持した親ハンドルから nofollow + create_new で作成する。
- 開いた祖先を rename してリンクへ置き換えてから書き込む決定的なテストで、リンク先は空のまま、保持したディレクトリだけに生成されることを確認した。雛形のサブディレクトリがリンクへ置き換わった場合も拒否する。
- `cargo test --test new_command` の全 8 テスト、追加の競合・置換テスト 2 件、Clippy が成功した。通常のネスト・空の現在ディレクトリの生成とビルド、破損リンク・非空の拒否も維持した。
- macOS で検証した。Windows での実行は既存の scaffold CI の検証待ち。
- 未クローズ。`Completed:` は更新していない。
