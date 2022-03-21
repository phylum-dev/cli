---
title: Quickstart
excerpt: This page will help you get started with Phylum CLI tool. You'll be up and running in a jiffy!
category: 61e72e3a50a88e001a92ee5d
---

<p align="center">
  <img height="100" src="https://phylum.io/logo/dark-bckg.svg">
</p>

---

# Introduction

![GitHub release (latest by date)](https://img.shields.io/github/v/release/phylum-dev/cli)
[![MIT License](https://img.shields.io/apm/l/atomic-design-ui.svg?)](https://github.com/tterb/atomic-design-ui/blob/master/LICENSEs)
![Test Status](https://github.com/phylum-dev/cli/actions/workflows/test.yml/badge.svg?branch=master)
[![README](https://img.shields.io/badge/docs-README-yellowgreen)](https://docs.phylum.io/docs/welcome)

The command line interface (CLI) allows users to submit their project package dependencies to [Phylum's](https://phylum.io) API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. For other platforms (e.g., Windows), binaries can easily be [built](https://docs.phylum.io/docs/building).

[![asciicast](https://asciinema.org/a/431262.svg)](https://asciinema.org/a/431262)

## Quickstart for Linux or macOS

1. Download and unzip the latest release package for your target:

   | Target | Package |
   | --- | --- |
   | x86_64-unknown-linux-musl | [phylum-linux-x86_64.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-linux-x86_64.zip) |
   | x86_64-apple-darwin | [phylum-macos-x86_64.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-macos-x86_64.zip) |
   | aarch64-apple-darwin | [phylum-macos-aarch64.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-macos-aarch64.zip) |

2. Confirm the signature of the `phylum` binary located within the archive with [minisign](https://jedisct1.github.io/minisign/) and the public key for Phylum

   ```sh
   $ minisign -Vm phylum -P RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf
   Signature and comment signature verified
   Trusted comment: Phylum - the future of software supply chain security
   ```
   
3. Run the installer script for installation

   ```
   ./install.sh
   ```

4. [Register](https://docs.phylum.io/docs/registration) for an account (if you don't already have one)

   ```
   phylum auth register
   ```

5. [Authenticate](https://docs.phylum.io/docs/authentication) with Phylum

   ```
   phylum auth login
   ```

6. [Create a new Phylum project](https://docs.phylum.io/docs/projects#creating-a-new-project) in your project directory

   ```
   phylum projects create <project-name>
   ```

7. [Submit your package lock file](https://docs.phylum.io/docs/analyzing-dependencies)

   ```
   phylum analyze <package-lock-file.ext>
   ```

---

## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
