---
title: Manifest Format
category: TODO
hidden: true
---

Extension metadata is defined in the `PhylumExt.toml` file, using the [TOML]
format. Every manifest file consists of the following sections:

 - [`name`] — Extension name
 - [`entry_point`] — Execution entry point
 - [`permissions`] — Permissions required for execution
     - [`[[read]]`] — Required read path permissions
     - [`[[write]]`] — Required write path permissions
     - [`[[env]]`] — Required environment variable permissions
     - [`[[run]]`] — Required process execution permissions
     - [`[[net]]`] — Required network domain permissions

[TOML]: https://toml.io
[`name`]: https://docs.phylum.io/docs/extensions_manifest#name
[`entry_point`]: https://docs.phylum.io/docs/extensions_manifest#entry-point
[`permissions`]: https://docs.phylum.io/docs/extensions_manifest#permissions
[`[[read]]`]: https://docs.phylum.io/docs/extensions_manifest#read
[`[[write]]`]: https://docs.phylum.io/docs/extensions_manifest#write
[`[[env]]`]: https://docs.phylum.io/docs/extensions_manifest#env
[`[[run]]`]: https://docs.phylum.io/docs/extensions_manifest#run
[`[[net]]`]: https://docs.phylum.io/docs/extensions_manifest#net

## Name

The extension name is used as the subcommand for executing the extension and
acts as an identifier when referring to it.

The name must use only lowercase alphanumeric characters, `-` or `_`.

## Entry Point

The entry point points to the file which should be loaded as the initial
JavaScript module. Generally this will be the `main.ts` file in the extension
root directory.

Phylum CLI extensions support both JavaScript and TypeScript out of the box,
transpiling TypeScript automatically before execution.

## Permissions

Since extensions are executed inside Deno's JavaScript sandbox, no external
effects can be performed without requesting the necessary permissions.

Users will be prompted to agree to these permissions during install, they are
later validated during execution relative to the active working directory when
running it.

### Read

Read permissions list file paths which can be read from by the extension.

Granting permissions to a directory will also allow the extension to access any
child directories and files inside them.

### Write

Write permissions list file paths which can be written to by the extension.

Granting permissions to a directory will also allow the extension to access any
child directories and files inside them.

### Env

Env permissions list environment variables which can be read by the extension.

### Run

Run permissions list executable paths which can be executed by the extension.

The executable paths take `$PATH` into account, so it is recommended to avoid
using absolute paths to improve portability.

The paths also need to match **exactly** with the process executed by the
extension. `/usr/bin/curl` cannot be executed when `curl` was requested as
permission and vice versa.

### Net

Net permissions list domains which can be accessed by the extension.

Network permissions only describe the domains and subdomains an extension has
access to, regardless of its path segments. Access to a domain does not
automatically grant access to all of its subdomains.

If the requested domain requests a redirect, you'll also require permissions to
access the redirect target. It's easiest to just specify the redirect target
directly when making requests.
