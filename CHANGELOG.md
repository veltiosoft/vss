# CHANGELOG

- CHANGE
    - 後方互換性のない変更
- UPDATE
    - 後方互換性がある変更
- ADD
    - 後方互換性がある追加
- FIX
    - バグ修正


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
