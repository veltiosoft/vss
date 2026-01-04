---
title: First Post
description: My first blog post
author: zztkm
pub_datetime: 2024-11-08
post_slug: first-post
---

# First Post

This is my first blog post using vss!

## What is vss?

vss is a static site generator written in Rust. It converts Markdown files into HTML using templates.

### Key Features

1. **Simple** - Easy to use and configure
2. **Fast** - Built with Rust for performance
3. **Flexible** - Customizable templates and layouts

## Example

Here's a simple code example:

```rust
use std::fs;

fn main() {
    let content = fs::read_to_string("example.md").unwrap();
    println!("{}", content);
}
```

## Links and Formatting

You can use **bold**, *italic*, and ~~strikethrough~~ text.

- [x] Implement build command
- [x] Add GFM support
- [ ] Add more features

## Conclusion

Thanks for reading this example post!

