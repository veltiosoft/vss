# CHANGELOG

v0.5.0 から書き始めました

## v0.5.0

- [ADD] yaml front matter の読み取りをサポートしました
- [CHANGE] yaml front matter の追加に伴い、テンプレートのレンダリング時に使われる変数に変更が入りました
    - [CHANGE] front matter が設定されている場合、title と description は front matter の値が使われます
    - [ADD] 以下の変数が追加されました
        - author
        - pubDatetime
        - postSlug
        - tags
        - emoji
