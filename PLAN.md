# vss 公式サイト刷新：1 ページ版

## Context

- 旧サイトの `site/` と専用のデプロイ設定は作業ツリーで削除済み。Cloudflare Pages の公開版はまだ稼働している。
- vss で生成するサイトとして作り直す。個別機能を網羅するのではなく、最初の 1 ページで「何をするツールか」「どう使い始めるか」「詳細はどこか」を伝える。
- スマホ・PC、システム設定を既定とする白・黒の配色に対応する。画像や装飾用のコピーは置かず、短い本文とコマンドで説明する。
- 新サイトは GitHub Pages のデフォルト URL `https://veltiosoft.github.io/vss/` で公開する。独自ドメインの設定や、旧 Cloudflare Pages・旧 URL の停止と転送は行わない。

## Approach

### ページに載せる内容

初めて vss を知る人が、上から読むだけで Markdown を書いて公開するまでの流れをつかめるようにする。見出しと本文は次の案をベースにする。

1. **vss** — 「vss は、Markdown から静的な Web サイトを作るツールです。本文は Markdown で書き、HTML と CSS で見た目を整えます。」
2. **インストール** — 「[GitHub Releases](https://github.com/veltiosoft/vss/releases/latest) から OS と CPU に合うファイルをダウンロードします。展開した `vss`（Windows は `vss.exe`）を PATH が通った場所に置き、`vss --version` で確認します。」
3. **サイトを作る** — 次のコマンドを載せる。「`http://localhost:8080` で確認します。`index.md` を編集し、ブラウザーを再読み込みすると変更を確認できます。」

   ```sh
   vss new my-site
   cd my-site
   vss serve
   ```

4. **公開する** — 「`vss.toml` の `base_url` に公開先の URL を設定します。`vss build` を実行し、生成された `dist/` の中身を静的ホスティングに公開します。」
5. **詳しい使い方** — 「設定と機能の詳細は [GitHub の README](https://github.com/veltiosoft/vss#readme) を参照してください。」

初版では設定項目の一覧、リリース記事、タグページ、デモ画像は載せない。

### 表示と実装
- vss、Markdown、Mustache、HTML、CSS、必要な範囲の素の JS を使う。UI フレームワーク、CDN、外部フォントや画像は使わない。
- `modern-web-guidance` の `html`・`accessibility`・`css`・`responsive-design` に基づき、`lang="ja"`、見出しと `main`、モバイルの viewport を明示する。スマホ・PC とも読み順は同じ 1 カラムとし、PC で本文幅を制限する。長いコマンドはページ全体ではなくコード枠内で扱う。
- `dark-mode` に基づき、HTML の `color-scheme` メタタグと CSS のルートに `light dark` を指定し、配色トークンを `prefers-color-scheme` で切り替える。初版では手動切替を設けず、読み込みや操作を JS に依存させない。

### GitHub Pages への公開

- GitHub Actions で同じコミットの vss を `cargo run --locked --manifest-path ../Cargo.toml -- build`（`site/` で実行）により使い、`site/dist/` だけを Pages の成果物にする。リリースの `latest` に暗黙に依存しない。
- [GitHub Pages のカスタムワークフロー](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)に沿い、`main` への push と手動実行に対応する。Pages の公開元は GitHub Actions を選び、デプロイジョブだけに `pages: write` と `id-token: write` を与える。
- `base_url` は GitHub Pages のデフォルト URL。CSS は `./styles/main.css` のように相対参照し、ローカルの `/` と公開先の `/vss/` で同じ HTML を使う。

## Files to modify

- `site/vss.toml`：公開先の `base_url` とサイト設定。
- `site/index.md`：1 ページの説明とコマンド。
- `site/layouts/default.html`：ページ構造と相対パスでの CSS 読み込み。
- `site/static/styles/main.css`：レスポンシブレイアウトと配色。
- `.github/workflows/deploy-pages.yaml`：ビルド、Pages 成果物アップロード、デプロイ。削除済みの Cloudflare Pages 用ワークフローは復活させない。
- `README.md`、`src/skeleton/README.md`：公式サイトへのリンクを新 URL に更新する。ルート README 内の旧サイトの個別ガイドへのリンクは、同じ README 内の説明で完結するよう整理する。
- `CHANGELOG.md`：`## main` にサイト刷新と公開先の変更を記録する。

## Reuse

- `src/skeleton/vss.toml` と `src/skeleton/layouts/default.html`：vss の最小設定と、`{{{contents}}}` で Markdown の本文を挿入する構造を参照する。
- `README.md`：`vss new`、`vss serve`、`vss build`、`dist/` の既存手順を短く整理して使う。旧 `/docs/skill.html` と `/docs/posts.html` へのリンク先の内容は、README の既存セクションで案内できる。

## Steps

- [x] vss の最小構成を作り、上記の本文を 1 ページにまとめる。
- [x] HTML レイアウト、レスポンシブ CSS、白・黒の配色を実装する。
- [x] GitHub Pages 用のワークフローを追加し、README のリンクと変更履歴を更新する。
- [x] ローカルでビルドと表示を検証する。リポジトリの Pages 設定変更・push・公開は別途指示を受けて実施する。

## Verification

- CI と同じコマンドでビルドし、HTML は `dist/index.html` の 1 ページのみで、CSS がコピーされ、画像や外部スタイルシートを参照しないことを確認する。
- `vss serve` と、生成物を `/vss/` 配下に置いたローカル配信の両方で、CSS が読み込まれ、リンクが意図した先を指すことを確認する。
- 幅 320px・390px・1280px と文字拡大時にページ全体が横スクロールしないことを確認する。ライト・ダーク両方で本文・リンク・コードのコントラスト、キーボードのフォーカス表示、JS 無効時の表示を確認する。
- 雛形の README の変更後に `cargo test --locked --test new_command` を実行する。
- 公開時は Pages のデプロイ結果がデフォルト URL を示し、公開版の HTML と CSS が取得できることを確認する。
