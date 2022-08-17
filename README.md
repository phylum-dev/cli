<p align="center">
  <img height="100" src="https://raw.githubusercontent.com/phylum-dev/cli/main/assets/dark-bckg.svg">
</p>

---

# Introduction

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/phylum-dev/cli)](https://github.com/phylum-dev/cli/releases/latest/)
[![MIT License](https://img.shields.io/github/license/phylum-dev/cli)](./LICENSE)
[![Test Status](https://github.com/phylum-dev/cli/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/phylum-dev/cli/actions/workflows/test.yml)
[![README](https://img.shields.io/badge/docs-README-blue)](https://docs.phylum.io/docs/welcome)

The command line interface (CLI) allows users to submit their project package dependencies to [Phylum's](https://phylum.io) API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. For other platforms (e.g., Windows), binaries can easily be [built](https://docs.phylum.io/docs/alternate_install).

[![asciicast](https://asciinema.org/a/431262.svg)](https://asciinema.org/a/431262)

## Quickstart for Linux or macOS

1. Download the latest release package and signature file for your target:

   | Target | Package |
   | --- | --- |
   | x86_64-unknown-linux-gnu | [phylum-x86_64-unknown-linux-gnu.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-unknown-linux-gnu.zip) <br /> [phylum-x86_64-unknown-linux-gnu.zip.minisig](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-unknown-linux-gnu.zip.minisig) |
   | aarch64-unknown-linux-gnu | [phylum-aarch64-unknown-linux-gnu.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-unknown-linux-gnu.zip) <br /> [phylum-aarch64-unknown-linux-gnu.zip.minisig](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-unknown-linux-gnu.zip.minisig) |
   | x86_64-apple-darwin | [phylum-x86_64-apple-darwin.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-apple-darwin.zip) <br /> [phylum-x86_64-apple-darwin.zip.minisig](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-apple-darwin.zip.minisig) |
   | aarch64-apple-darwin | [phylum-aarch64-apple-darwin.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-apple-darwin.zip) <br /> [phylum-aarch64-apple-darwin.zip.minisig](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-apple-darwin.zip.minisig) |

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

1. [Register](https://docs.phylum.io/docs/phylum_auth_register) for an account (if you don't already have one)

   ```
   phylum auth register
   ```

1. [Authenticate](https://docs.phylum.io/docs/phylum_auth_login) with Phylum

   ```
   phylum auth login
   ```

1. [Create a new Phylum project](https://docs.phylum.io/docs/phylum_project_create) in your project directory

   ```
   phylum project create <project-name>
   ```

1. [Submit your package lock file](https://docs.phylum.io/docs/phylum_analyze)

   ```
   phylum analyze <package-lock-file.ext>
   ```

 1. (Optional) View the analysis results in the [Phylum UI](https://app.phylum.io/auth/login)

## Extensions

Phylum CLI extensions allow you to extend the existing CLI functionality with
new features. You can start exploring by taking a look at the official Phylum
extensions:

https://github.com/phylum-dev/cli/tree/main/extensions

### musl binaries

As of version 3.8.0, the provided Linux binaries of the Phylum CLI depend on
`glibc`. We no longer provide binaries that are statically compiled with the
`musl` libc.

This means the provided binaries won't be executable in environments such as
Alpine Linux. If your use case requires a lightweight Docker base image,
consider using [Debian slim](debian-slim) instead.

[debian-slim]: https://hub.docker.com/_/debian

---
## Slack

Join us on the [Phylum Community Slack](https://join.slack.com/t/phylumio/shared_invite/zt-1cbgl6qjp-C_mkSFibEA9DyDxjYHbttQ)!


## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
