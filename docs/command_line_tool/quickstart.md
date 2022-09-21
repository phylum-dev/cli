---
title: Quickstart
excerpt: This page will help you get started with Phylum CLI tool. You'll be up and running in a jiffy!
category: 6255e67693d5200013b1fa3e
hidden: false
---

<p align="center">
  <img height="100" src="https://raw.githubusercontent.com/phylum-dev/cli/main/assets/dark-bckg.svg">
</p>

---

# Introduction

The command line interface (CLI) allows users to submit their project package dependencies to [Phylum's](https://phylum.io) API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. For other platforms (e.g., Windows), binaries can easily be [built](https://docs.phylum.io/docs/alternate_install).

[![asciicast](https://asciinema.org/a/431262.svg)](https://asciinema.org/a/431262)

## Quickstart for Linux or macOS

1. Download the latest release package and signature file for your target:

   | Target | Package |
   | --- | --- |
   | x86_64-unknown-linux-musl | [phylum-x86_64-unknown-linux-musl.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-unknown-linux-musl.zip) <br /> [phylum-x86_64-unknown-linux-musl.zip.minisig](https://github.com/phylum-dev/cli/releases/latest/download/phylum-x86_64-unknown-linux-musl.zip.minisig) |
   | aarch64-unknown-linux-musl | [phylum-aarch64-unknown-linux-musl.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-unknown-linux-musl.zip) <br /> [phylum-aarch64-unknown-linux-musl.zip.minisig](https://github.com/phylum-dev/cli/releases/latest/download/phylum-aarch64-unknown-linux-musl.zip.minisig) |
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

---
## License

Copyright (C) 2022  Phylum, Inc.

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License or any later version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program. If not, see <https://www.gnu.org/licenses/gpl.html> or write to
`phylum@phylum.io` or `engineering@phylum.io`

## Slack

Join us on the [Phylum Community Slack](https://join.slack.com/t/phylumio/shared_invite/zt-1cbgl6qjp-C_mkSFibEA9DyDxjYHbttQ)!

## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
