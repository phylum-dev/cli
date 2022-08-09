---
title: Alternate Installation Methods
category: 6255e67693d5200013b1fa3e
hidden: false
---

## Manual download

1. Download the release package and minisign file for your architecture from our latest [release on GitHub](https://github.com/phylum-dev/cli/releases)

1. Confirm the signature of the archive with [minisign](https://jedisct1.github.io/minisign/) and the public key for Phylum

   ```sh
   $ minisign -Vm phylum-*.zip -P RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf
   Signature and comment signature verified
   Trusted comment: Phylum - The Software Supply Chain Company
   ```

1. Unzip the archive

   ```sh
   unzip phylum-*.zip
   ```

1. Run the installer script for installation

   ```sh
   ./install.sh
   ```

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
