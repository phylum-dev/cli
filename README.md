<p align="center">
  <img height="100" src="https://raw.githubusercontent.com/phylum-dev/cli/main/assets/dark-bckg.svg">
</p>

---

# Introduction

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/phylum-dev/cli)](https://github.com/phylum-dev/cli/releases/latest/)
[![MIT License](https://img.shields.io/github/license/phylum-dev/cli)](./LICENSE)
[![Test Status](https://github.com/phylum-dev/cli/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/phylum-dev/cli/actions/workflows/test.yml)
[![README](https://img.shields.io/badge/docs-README-blue)](https://docs.phylum.io/docs/welcome)

The command line interface (CLI) allows users to submit their project package dependencies to [Phylum's](https://phylum.io) API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. For other platforms (e.g., Windows), binaries can easily be [built](https://docs.phylum.io/docs/building).

[![asciicast](https://asciinema.org/a/431262.svg)](https://asciinema.org/a/431262)

## Quickstart for Linux or macOS

1. Download the latest release package for your target:

   | Target | Package |
   | --- | --- |
   | x86_64-unknown-linux-musl | [phylum-x86_64-unknown-linux-musl.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-unknown-linux-musl.zip) |
   | x86_64-apple-darwin | [phylum-x86_64-apple-darwin.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-apple-darwin.zip) |
   | aarch64-apple-darwin | [phylum-aarch64-apple-darwin.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-apple-darwin.zip) |

1. Confirm the signature of the archive with [minisign](https://jedisct1.github.io/minisign/) and the public key for Phylum

   ```sh
   $ minisign -Vm phylum-*.zip -P RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf
   Signature and comment signature verified
   Trusted comment: Phylum - the future of software supply chain security
   ```

1. Unzip the archive

   ```sh
   unzip phylum-*.zip
   ```

1. Run the installer script for installation

   ```
   ./install.sh
   ```

1. [Register](https://docs.phylum.io/docs/registration) for an account (if you don't already have one)

   ```
   phylum auth register
   ```

1. [Authenticate](https://docs.phylum.io/docs/authentication) with Phylum

   ```
   phylum auth login
   ```

1. [Create a new Phylum project](https://docs.phylum.io/docs/projects#creating-a-new-project) in your project directory

   ```
   phylum project create <project-name>
   ```

1. [Submit your package lock file](https://docs.phylum.io/docs/analyzing-dependencies)

   ```
   phylum analyze <package-lock-file.ext>
   ```

 1. (Optional) View the analysis results in the [Phylum UI](https://app.phylum.io/auth/login)

---

## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
