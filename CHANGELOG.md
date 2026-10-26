# CHANGELOG

- CHANGE
  - 後方互換性のない変更
- UPDATE
  - 後方互換性がある変更
- ADD
  - 後方互換性がある追加
- FIX
  - バグ修正

## main

- [CHANGE] `dist` をサイト内の専用ディレクトリを指す相対パスに限定する
  - サイト外・サイトルート・入力ディレクトリ・Git ディレクトリと重なるパスや、シンボリックリンクを経由するパスは、`build` と `serve` の削除・書き込み前に拒否する。
  - 絶対パスや `..` を含む設定は、`dist = "public"` などのサイト内のパスへ変更する。
- [CHANGE] `static` と `layouts` をサイト内の相対パスに限定し、静的ファイルの隠しパスを公開対象から除外する
  - サイト外・サイトルート・出力先と重なる入力は、`build` と `serve` の出力削除前に拒否する。サイト外入力を使う設定はサイト内へ移す。
  - 静的ファイルはルート直下の `.well-known/` のみ隠しパスの例外とし、その配下を含め `.env`、`.git/`、`.ssh/` などは全体・増分ビルドともコピーしない。
  - 入力ルートに指定された機密ディレクトリは、Windows の末尾ドット・空白や実体が同じ別名も拒否する。
- [CHANGE] インストーラーは SHA-256 チェックサムを検証してから必要なバイナリだけを配置する
  - ダウンロードとリダイレクトを HTTPS に限定し、検証失敗時は既存バイナリを保持する。チェックサムのない旧リリースはこのインストーラーではインストールできない。
  - PowerShell 版の配布元と ZIP 内の実行ファイルのパスを現行リリースに合わせる。
- [CHANGE] Windows で無効なタグ名をすべての OS でタグリンク・タグページの公開対象から除外する
  - 無効文字、末尾のドット・空白、予約デバイス名を含むタグはビルドを失敗させず無視する。名前の置換は行わず、英数字・日本語・`C++`・`a b` などの既存の URL と出力先を維持する。
- [UPDATE] 自己更新とサーバーの依存を既知の脆弱性の修正版へ更新し、CI に `cargo audit` を追加する
  - `self_update` を更新し、`ureq` と `rustls`、`webpki-roots` による TLS 検証を使用する。
  - frontmatter の YAML パーサを `serde-saphyr` に移行し、未メンテナンスの `serde_yaml` / `unsafe-libyaml` への依存を除く。
- [FIX] リリース時のタグ名を環境変数経由で渡し、シェルへのコマンド注入を防ぐ
  - リリースタグの書式を検証し、ジョブの権限を限定し、使用アクションを commit SHA に固定する。
  - 配布アーカイブの SHA-256 チェックサムと GitHub Artifact Attestations を生成する。
- [FIX] `serve` で許可外・欠落・重複した Host ヘッダを拒否し、セキュリティヘッダを付与する
  - `localhost` と `127.0.0.1` のみを許可し、ルート直下の `.well-known/` を除く隠しパスは配信しない。既定のバインド先は `127.0.0.1` のままとする。
- [FIX] `new` で生成先の祖先にシンボリックリンクがある場合も拒否する
  - ディレクトリハンドルと nofollow の相対操作で雛形を作成し、検査後のリンク置換による書き込み先の逸脱を防ぐ。
- [FIX] `self update` の未対応 OS・アーキテクチャで panic せず、対応状況を示すエラーを返す
- [FIX] Rust 版の `new` コマンドでビルド可能なサイトの雛形を生成する
  - 存在しないディレクトリと空の既存ディレクトリに対応し、`vss new .` でも生成できるようにする。
  - 空でないディレクトリへの生成は拒否し、既存ファイルを上書きしない。
  - 生成するテキストファイルの文字コードを UTF-8、改行を LF に統一する。
- [FIX] frontmatter の `og_image` をページテンプレートへ渡すようにする
  - URL を解決した `og_image_url` を追加し、HTTP(S) の絶対 URL とサイト内の画像パスに対応する。
  - 投稿テンプレートは画像未指定時に `og:image` を出力しない。

## v0.21.0

- [CHANGE] Markdown の自動探索で `.` 始まりのディレクトリを除外する
  - ページと Markdown 部品が対象。`.claude/` や `.github/`、入れ子のドットディレクトリ内の Markdown をページ化・部品化・タグ一覧への掲載対象にしない。
  - 静的ファイルとテンプレートは従来どおり読み込み、`static/.well-known/` もコピーする。サイト自体のディレクトリ名や Markdown ファイル名が `.` 始まりの場合は除外しない。
- [CHANGE] 入力のシンボリックリンクをビルド対象から除外する
  - Markdown、部品、テンプレート、静的ファイルと、それらのディレクトリへのリンクを辿らない。必要な入力は通常のファイルとして配置する。
- [CHANGE] タグのリンクをサイトルートからの URL に統一し、特殊文字をパーセントエンコードする
  - コロンと制御文字を含むタグは公開しない。タグの URL パターンにはサイト内のパスを指定する。
- [CHANGE] YAML frontmatter を 64 KiB、タグを 1 ファイル 128 個・1 個 256 バイトまでに制限する
  - 上限を超える入力や不正な frontmatter はビルドエラーにする。本文のサイズはこの制限の対象外とする。
- [FIX] 自己更新で使用する TLS 依存を更新し、信頼を取り消された証明書を拒否する
- [FIX] 入力のシンボリックリンクを経由したサイト外のファイルの公開を防ぐ
  - `serve` 中に入力がリンクへ置き換わった場合は全体を再ビルドし、以前の生成物も削除する。
- [FIX] タグ名から JavaScript URL が生成される問題を修正する
- [FIX] YAML の参照展開とタグのグループ化によるメモリ消費の増幅を防ぐ
  - frontmatter は必要な型へ直接読み込み、タグページでは投稿メタデータを複製せず共有する。

## v0.20.0

- [ADD] `serve` コマンドで初回ビルドおよび再ビルド時に処理時間を表示する
- [UPDATE] `serve` コマンドで変更内容に応じた増分ビルドを行うようにする
  - Markdown は変更されたページ、静的ファイルは変更されたファイル、テンプレートは利用するページを更新する。
  - 設定・Markdown 部品・ディレクトリ構成の変更や不要になったタグページがある場合は全体を再ビルドする。
  - 削除・リネーム後の生成物を更新し、設定変更後も新しい静的ファイル・テンプレート・配信先を使用する。
- [FIX] タグ名のパストラバーサルによる出力先の逸脱を防ぐ

## v0.19.1

- [FIX] Immutable Releases が有効な場合に配布バイナリを追加できない問題を修正
  - ドラフトリリースに全プラットフォームの成果物を追加し、すべてのビルドが成功した後で正式公開する

## v0.19.0

- [ADD] `partials/` の Markdown をテンプレートに差し込めるようにする
  - `{{{partials.news}}}` で HTML を参照し、`{{#partials.news}}…{{/partials.news}}` で空のときは欄ごと非表示にできる
  - 部品は通常ページ・タグページから参照でき、単独のページやタグ一覧の投稿としては出力しない
  - [#51](https://github.com/veltiosoft/vss/issues/51)

- [FIX] `serve` コマンドで `ls` 実行時にビルドが誤って走る問題を修正
  - `notify-debouncer-mini` から `notify-debouncer-full` に移行し、`EventKind::Access` を除外するようにした
  - [#46](https://github.com/veltiosoft/vss/pull/46)

## v0.18.0

- [ADD] tag ページ生成機能 (仮) を追加する
  - 実験的機能であるため、破壊的変更が行われる可能性があることに注意
  - @zztkm

## v0.17.0

- [ADD] 設定に `build.markdown.allow_dangerous_html` を追加する
  - この設定は vss が依存する markdown-rs の `allow_dangerous_html` を有効化するためのオプション
  - `allow_dangerous_html` に関するセキュリティリスクについては https://github.com/wooorm/markdown-rs/tree/1.0.0?tab=readme-ov-file#security を確認
  - @zztkm

### misc

- [UPDATE] `.github/workflows/update-docs.yaml` で利用する vss バイナリの取得は GitHub Release の latest から取得するように変更する
  - Go 実装のときにはビルド速度のおかげでドキュメント更新も時間がかからなかったが、Rust でのビルドには以前より時間がかかる
  - そのため、毎回ビルドするより latest から取得してくるようにした
  - @zztkm
- [UPDATE] `.github/workflows/update-docs.yaml` の runner 指定を ubuntu-latest から ubuntu-24.04 に変更する
  - @zztkm

## v0.16.0

- [UPDATE] 実装言語を Rust に移行する
  - 基本仕様の変更はない
  - @zztkm

## v0.14.0

- [UPDATE] 依存関係をアップデート
  - @zztkm

## v0.13.0

- [CHANGE] LICENSE を Apache License 2.0 に変更
  - @zztkm

## v0.12.0

- [UPDATE] ビルド設定に `build.goldmark.renderer` を追加しました
  - 現在サポートするオプションは `with_unsafe` のみ
  - `with_unsafe` は `true` または `false` を指定できる
  - `true` にすると、生の HTML せずにそのままレンダリングする
  - 詳しくは [goldmark のドキュメント](https://github.com/yuin/goldmark?tab=readme-ov-file#html-renderer-options) を参照

## v0.11.1

- [FIX] ignore_files の設定が機能していなかった問題を修正しました
  - @zztkm

### misc

- [CHANGE] organization を vssio から veltiosoft に変更しました
  - これに伴い、Go パッケージ `github.com/vssio/go-vss` は `github.com/veltiosoft/go-vss` に変更
  - @zztkm
- [UPDATE] パッケージ構成をフラットパッケージに変更しました
  - @zztkm
- [UPDATE] GitHub Actions に ci.yaml を追加しました
  - @zztkm

## v0.11.0

- [FIX] vss self update の最後に実行する `vss self version` が失敗する問題を修正しました

## v0.10.0

- [FIX] goroutine でコンテンツを生成するときに意図しないデータの上書きが発生し、ビルド結果が正しくないことがある問題を修正しました

## v0.9.0

- [UPDATE] コンテンツの生成を goroutine で行うようにしました
  - これにより、ビルド時間が短縮されることが期待されます

## v0.8.0

- [CHANGE] Yaml front matter のキーの命名規則を snake_case に変更しました
  - これに伴い、テンプレートのレンダリング時に使われる変数も変更されました
  - 今までは front matter が camelCase で vss.toml が snake_case でしたが、これはユーザーの認知負荷が高いと判断したので統一しました
- [CHANGE] vss.toml のキー名を変更しました
  - `title` を `site_title` に変更しました
  - `description` を `site_description` に変更しました
  - これに伴い、テンプレートのレンダリング時に使われる変数も変更されました
  - title と description は yaml front matter で使われているため、衝突を避けるために変更しました
- [UPDATE] Yaml front matter に `og_image` 変数を追加しました
  - og:image の設定に利用してください
- [UPDATE] Yaml front matter に `emoji` が設定されていて、かつ `og_image` が設定されていない場合、`og_image` 変数には emoji の画像パスが設定されるようになりました
  - og_image が設定されていない場合と判定される条件は以下の通りです
    - og_image が空文字列の場合
    - og_image が設定されていない場合

## v0.7.0

- [UPDATE] 設定に `build.goldmark.highlight` を追加しました
  - この設定を加えるとハイライト済みのコードブロックを生成するようになります
  - 使っているライブラリはこちら: <https://github.com/yuin/goldmark-highlighting>
  - 対応言語やスタイルについてはこちら: <https://github.com/alecthomas/chroma>
  - このバージョンでサポートしているキー
    - `style` (string)
    - `with_numbers` (bool)
- [FIX] vss serve 時に dist ディレクトリも監視対象に含んでしまい、無限ループに陥る問題を修正しました [#21](https://github.com/vssio/go-vss/issues/21)

## v0.6.0

空白の100年

- <https://github.com/vssio/go-vss/releases/tag/v0.6.0>

## v0.5.0

- [ADD] yaml front matter の読み取りをサポートしました (front matter は必須ではありません)
- [UPDATE] yaml front matter の追加に伴い、テンプレートのレンダリング時に使われる変数に変更が入りました
  - [UPDATE] front matter が設定されている場合、title と description は front matter の値が使われます
  - [ADD] 以下の変数が追加されました
    - author
    - pubDatetime
    - postSlug
    - tags
    - emoji
