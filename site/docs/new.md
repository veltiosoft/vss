---
title: サイトの雛形を作成する
description: vss new で空のディレクトリにサイトを作成する方法
---

# サイトの雛形を作成する

`vss new <DIR>` は、設定ファイル、Markdown、テンプレート、CSS を含む最小サイトを生成します。

```sh
vss new my-site
cd my-site
vss build
vss serve
```

`build` の生成物は `dist/` に出力されます。
`serve` の既定のポートは `8080` です。
ファイルを編集すると自動で再ビルドしますが、ブラウザーのリロードは手動です。

## 既存の空ディレクトリに生成する

現在のディレクトリが空なら、`.` を指定できます。

```sh
mkdir my-site
cd my-site
vss new .
```

空でないディレクトリ、通常ファイル、シンボリックリンクを生成先に指定するとエラーになります。
ドットファイルや空のサブディレクトリも「空ではない」と判定します。
たとえば `.git/` があるディレクトリには生成できません。
上書きオプションは提供していないため、別の空ディレクトリを指定してください。

引数やヘルプは `vss new --help` で確認できます。
生成するテキストファイルは、OS にかかわらず UTF-8 と LF 改行を使用します。

## 生成するファイル

```text
my-site/
├── .gitignore
├── README.md
├── vss.toml
├── index.md
├── layouts/
│   ├── default.html
│   └── tags/
│       └── default.html
└── static/
    └── style.css
```

`index.md` を編集し、`layouts/` に HTML テンプレート、`static/` に画像や CSS を置きます。
`README.md` は `build.ignore_files` でビルド対象から除外しています。
`dist/` は生成した `.gitignore` で Git の管理対象から除外しています。

## 公開する

`vss.toml` の `base_url` に HTTP(S) の公開先 URL を設定してから、`vss build` を実行します。
公開するのは `dist/` の中身です。
サイト内の Markdown は再帰的に探索されるため、公開したくないファイルをサイトのディレクトリに置かないでください。
