---
# Make this a docs-only site, with this doc as the front page
slug: /
---

# Quickstart

The Phylum command line interface (CLI) allows users to submit their project package dependencies to [Phylum's](https://phylum.io) API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. On Windows, we recommend using the Linux binaries under [WSL](https://learn.microsoft.com/en-us/windows/wsl/). For more options, see [Alternate Installation Methods](https://docs.phylum.io/docs/alternate_install).

[![asciicast](https://asciinema.org/a/431262.svg)](https://asciinema.org/a/431262)

## Install phylum

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

:::info

When using Homebrew, [official extensions][] must be installed separately.

:::

[official extensions]: https://github.com/phylum-dev/cli/tree/main/extensions

## Quickstart for Linux or macOS

1. [Register](https://docs.phylum.io/docs/phylum_auth_register) for an account (if you don't already have one)

   ```sh
   phylum auth register
   ```

1. [Authenticate](https://docs.phylum.io/docs/phylum_auth_login) with Phylum

   ```sh
   phylum auth login
   ```

1. [Setup your Phylum project](https://docs.phylum.io/docs/phylum_init) in your project directory

   ```sh
   phylum init
   ```

1. [Submit your package lock file](https://docs.phylum.io/docs/phylum_analyze)

   ```sh
   phylum analyze
   ```

1. (Optional) View the analysis results in the [Phylum UI](https://app.phylum.io/auth/login)

---
## License

Copyright (C) 2023  Phylum, Inc.

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License or any later version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program. If not, see <https://www.gnu.org/licenses/gpl.html> or write to
`phylum@phylum.io` or `engineering@phylum.io`

## Discord

Join us on the [Phylum Community Discord](https://discord.gg/c9QnknWxm3)!

## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
