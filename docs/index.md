# vss documentation

A static site generator.

![vss-logo](./image.gif)

- [GitHub](https://github.com/vssio/go-vss)

## Features

- Write content in Markdown
  - GFM (GitHub Flavored Markdown) is supported
- Render HTML with [Mustache](https://github.com/cbroglie/mustache)
- Local development server (Rebuilds when files are modified)
    - But now, page reloading must be done manually

## Installation

For Mac & Linux

latest version
```sh
curl -sfL https://raw.githubusercontent.com/vssio/go-vss/main/scripts/install.sh | sh
```

specific version
```sh
curl -sfL https://raw.githubusercontent.com/vssio/go-vss/main/scripts/install.sh | sh -s v0.0.1
```

For Windows (with PowerShell)
```powershell
irm https://raw.githubusercontent.com/vssio/go-vss/main/scripts/install.ps1 | iex
```

specific version
```powershell
$v="v0.0.1"; irm https://raw.githubusercontent.com/vssio/go-vss/main/scripts/install.ps1 | iex
```

## Contents

- [about page](./about)
- post
  - [first](./post/first)