---
title: Welcome
description: Welcome to the example site
---

<!-- This is an HTML comment example -->

# Welcome to vss Example Site

This is an example static site built with **vss** (Veltiosoft Static Site generator).

<!--
  Multi-line HTML comment example:
  This comment spans multiple lines
  and can contain useful notes for developers
-->

## Features

- **GitHub Flavored Markdown** support
- **Mustache** template engine
- **Static file** serving
- **YAML frontmatter** support

## Getting Started

To build this site:

```bash
cd example_site
cargo run -- build
```

The generated files will be in the `dist/` directory.

## Example Content

Here's a list of features:

- Markdown to HTML conversion
- Template-based layouts
- CSS styling
- Fast build times

### Code Highlighting

```rust
fn main() {
    println!("Hello from vss!");
}
```

### Tables

| Feature           | Status |
|-------------------|--------|
| Markdown parsing  | ✓      |
| Templates         | ✓      |
| Static files      | ✓      |
| Frontmatter       | ✓      |

### Page Link

[First blog post](/posts/first)

