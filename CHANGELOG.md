# CHANGELOG

- CHANGE
    - 後方互換性のない変更
- UPDATE
    - 後方互換性がある変更
- ADD
    - 後方互換性がある追加
- FIX
    - バグ修正

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
    - 使っているライブラリはこちら: https://github.com/yuin/goldmark-highlighting
    - 対応言語やスタイルについてはこちら: https://github.com/alecthomas/chroma
    - このバージョンでサポートしているキー
        - `style` (string)
        - `with_numbers` (bool)
- [FIX] vss serve 時に dist ディレクトリも監視対象に含んでしまい、無限ループに陥る問題を修正しました [#21](https://github.com/vssio/go-vss/issues/21)

## v0.6.0

空白の100年
- https://github.com/vssio/go-vss/releases/tag/v0.6.0

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
