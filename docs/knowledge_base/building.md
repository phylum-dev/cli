---
title: Building
category: 6255e67693d5200013b1fa41
hidden: false
---

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
