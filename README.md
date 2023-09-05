<p align="center">
  <img height="100" src="https://raw.githubusercontent.com/phylum-dev/cli/main/assets/dark-bckg.svg">
</p>

---

# Introduction

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/phylum-dev/cli)](https://github.com/phylum-dev/cli/releases/latest/)
[![License](https://img.shields.io/github/license/phylum-dev/cli)](https://github.com/phylum-dev/cli/blob/main/LICENSE)
[![Test Status](https://github.com/phylum-dev/cli/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/phylum-dev/cli/actions/workflows/test.yml)
[![README](https://img.shields.io/badge/docs-README-blue)](https://docs.phylum.io/docs/welcome)

The Phylum command line interface (CLI) allows users to submit their project package dependencies to [Phylum's](https://phylum.io) API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. On Windows, we recommend using the Linux binaries under [WSL](https://learn.microsoft.com/en-us/windows/wsl/). For more options, see [Alternate Installation Methods](https://docs.phylum.io/docs/alternate_install).

## Install `phylum` CLI

### Install on Linux

Install on Linux with the following command:

```sh
curl https://sh.phylum.io/ | sh -
```

### Install on macOS

On macOS, we recommend installing phylum with [Homebrew](https://brew.sh/):

```sh
brew tap phylum-dev/cli
brew install phylum
```

_Note: When using Homebrew, [official extensions][] must be installed separately._

[official extensions]: https://github.com/phylum-dev/cli/tree/main/extensions

## Quickstart for Linux or macOS

1. [Register](https://docs.phylum.io/docs/phylum_auth_register) for an account (if you don't already have one)

   ```
   phylum auth register
   ```

1. [Authenticate](https://docs.phylum.io/docs/phylum_auth_login) with Phylum

   ```
   phylum auth login
   ```

1. [Setup your Phylum project](https://docs.phylum.io/docs/phylum_init) in your project directory

   ```
   phylum init
   ```

1. [Submit your package lock file](https://docs.phylum.io/docs/phylum_analyze)

   ```
   phylum analyze
   ```

1. (Optional) View the analysis results in the [Phylum UI](https://app.phylum.io/auth/login)

## Extensions

Phylum CLI extensions allow you to extend the existing CLI functionality with
new features. You can start exploring by taking a look at the official Phylum
extensions:

<https://github.com/phylum-dev/cli/tree/main/extensions>

### How-tos

How-to articles for the extension framework can be found [here](https://dev.to/phylum).

## musl binaries

As of version 3.8.0, the provided Linux binaries of the Phylum CLI depend on
`glibc`. We no longer provide binaries that are statically compiled with the
`musl` libc.

This means the provided binaries won't be executable in environments such as
Alpine Linux. If your use case requires a lightweight Docker base image,
consider using [Debian slim][debian-slim] instead.

[debian-slim]: https://hub.docker.com/_/debian

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

---
## Discord

Join us on the [Phylum Community Discord](https://discord.gg/c9QnknWxm3)!

## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
