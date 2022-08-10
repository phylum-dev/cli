---
title: Alternate Installation Methods
category: 6255e67693d5200013b1fa41
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
   cargo install --path cli
   ```

## Install with phylum-ci

See the [phylum-ci](https://github.com/phylum-dev/phylum-ci) project for details.

```sh
pipx install phylum-ci
phylum-init
```

## Install with curl

This script requires `curl`, `mktemp`, `unzip`, and [`minisign`](https://jedisct1.github.io/minisign/):

```sh
curl https://sh.phylum.io/ | sh -
```
