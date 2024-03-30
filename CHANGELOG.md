# CHANGELOG

- CHANGE
    - 後方互換性のない変更
- UPDATE
    - 後方互換性がある変更
- ADD
    - 後方互換性がある追加
- FIX
    - バグ修正


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
