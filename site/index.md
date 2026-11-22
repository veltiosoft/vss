# vss

vss は、Markdown から静的な Web サイトを作るツールです。
本文は Markdown で書き、HTML と CSS で見た目を整えます。

## インストール

[GitHub Releases](https://github.com/veltiosoft/vss/releases/latest) から OS と CPU に合うファイルをダウンロードします。
展開した `vss`（Windows は `vss.exe`）を PATH が通った場所に置き、`vss --version` で確認します。

## サイトを作る

```sh
vss new my-site
cd my-site
vss serve
```

[http://localhost:8080](http://localhost:8080) でサイトを確認します。
`index.md` を編集し、ブラウザーを再読み込みすると変更を確認できます。

## 公開する

開発サーバーを止め、`vss.toml` の `base_url` に公開先の URL を設定します。

```sh
vss build
```

生成された `dist/` の中身を静的ホスティングに公開します。

## クックブック

[やりたいことから設定例を探す](./cookbook.html)

## vss で作られたサイト

- [Pistachio Studio](https://pistachiostudio.net/)
- [quojama.com](https://quojama.com/)

## 詳しい使い方

設定と機能の詳細は [GitHub の README](https://github.com/veltiosoft/vss#readme) を参照してください。
