# タグページの投稿を pub_datetime でソートする

## 現状

タグページ（例: `/tags/blog/index.html`）に表示される投稿リストは、現在ソートされていない状態で表示されている。

### 該当コード

`src/subcommand_build.rs` の `generate_tag_pages` 関数（lines 470-530）

```rust
// タグごとに投稿をグループ化
let mut tag_to_posts: HashMap<String, Vec<PostMetadata>> = HashMap::new();

for post in all_posts {
    if let Some(tags_vec) = &post.tags {
        for tag_name in tags_vec {
            tag_to_posts
                .entry(tag_name.clone())
                .or_default()
                .push(post.clone());
        }
    }
}

// 各タグのページを生成
for (tag_name, posts) in tag_to_posts {
    let context = TagPageContext {
        site_title: config.site_title.clone(),
        site_description: config.site_description.clone(),
        base_url: config.base_url.clone(),
        tag_name: tag_name.clone(),
        posts,  // ← ここでソートされていない posts をそのまま渡している
    };
    // ...
}
```

## 課題

- タグページに表示される投稿が追加された順序で表示されてしまい、時系列順に並んでいない
- 新しい投稿が古い投稿の下に表示されてしまう可能性がある
- ユーザーが最新の投稿を見つけにくい

## 期待される動作

タグページに表示される投稿リストが `pub_datetime` フィールドで降順（新しい順）にソートされている。

例：
```
/tags/blog/
  - 2024-11-08: Second Post (最新)
  - 2024-11-01: First Post
```

## 実装案

### 案1: TagPageContext に渡す前にソートする

`generate_tag_pages` 関数内で、各タグの投稿リストを `pub_datetime` でソートしてから `TagPageContext` に渡す。

```rust
for (tag_name, mut posts) in tag_to_posts {
    // pub_datetime で降順ソート（新しい順）
    posts.sort_by(|a, b| b.pub_datetime.cmp(&a.pub_datetime));

    let context = TagPageContext {
        site_title: config.site_title.clone(),
        site_description: config.site_description.clone(),
        base_url: config.base_url.clone(),
        tag_name: tag_name.clone(),
        posts,
    };
    // ...
}
```

### 案2: 設定ファイルでソート順を指定可能にする

`vss.toml` の `[build.tags]` セクションに `sort_by` と `sort_order` フィールドを追加し、ユーザーがソート方法を選択できるようにする。

```toml
[build.tags]
enable = true
template = "tags/default.html"
url_pattern = "/tags/{tag}/"
sort_by = "pub_datetime"  # "pub_datetime", "title", "none" など
sort_order = "desc"        # "asc" または "desc"
```

## 技術的な考慮事項

- `pub_datetime` は現在 `String` 型なので、文字列比較になる（YYYY-MM-DD 形式なら辞書順でソート可能）
- より正確なソートを行う場合は、`pub_datetime` を `chrono::NaiveDate` などの日付型にパースする必要がある
- パース失敗時のエラーハンドリングを考慮する必要がある

## 関連ファイル

- `src/subcommand_build.rs`: タグページ生成ロジック（lines 470-530）
- `src/subcommand_build.rs`: `PostMetadata` 構造体定義（lines 135-143）
- `example_site/vss.toml`: タグページ設定（lines 13-23）

## 優先度

中 - ユーザービリティの改善

## ラベル

- enhancement
- タグ機能
- ソート
