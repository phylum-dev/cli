---
title: Alternate Installation Methods
category: 6255e67693d5200013b1fa3e
hidden: false
---

## Build from source

Phylum is written in Rust, so you'll need a recent Rust installation to build it (we tested with v1.61.0). [Install Rust](https://www.rust-lang.org/tools/install)

1. Clone repository

   ```sh
   git clone https://github.com/phylum-dev/cli
   ```

2. Build the project

   ```sh
   cargo build
   ```

3. You can use the executable directly as `./target/debug/phylum` or install it like so:

   ```sh
   cargo install --locked --path cli
   ```

## Install with Python

The [`phylum` Python package](https://pypi.org/project/phylum/) provides a script entry point, `phylum-init`, for bootstrapping the Phylum CLI.
See the [phylum-ci](https://github.com/phylum-dev/phylum-ci) project for full detail.

```sh
pipx install phylum
phylum-init
```

## Install with curl

This script requires `curl`, `mktemp`, `unzip`, and `openssl`:

```sh
curl https://sh.phylum.io/ | sh -
```
