# Alternate Installation Methods

## Build from source

Phylum is written in Rust, so you'll need a recent Rust installation to build it (we tested with v1.61.0). See [how to install Rust](https://www.rust-lang.org/tools/install) for more information.

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

## Install from GitHub release

Pre-built binaries for select targets are available in our [GitHub releases](https://github.com/phylum-dev/cli/releases).

Archives can be manually verified by their signature file (which has the `.signature` extension) with `openssl` and the [public key for Phylum](https://raw.githubusercontent.com/phylum-dev/cli/main/scripts/signing-key.pub):

```sh
$ openssl dgst -sha256 -verify signing-key.pub -signature phylum-*.zip.signature phylum-*.zip
Verified OK
```
