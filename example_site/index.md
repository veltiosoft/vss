---
title: Welcome
description: Welcome to the example site
---

# Welcome to vss Example Site

This is an example static site built with **vss** (Veltiosoft Static Site generator).

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

