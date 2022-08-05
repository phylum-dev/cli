---
title: Extension Manifest Format
category: 62c5cb137dbdad00536291a6
hidden: false
---

## Overview

Extension metadata is defined in the `PhylumExt.toml` file, using the [TOML]
format. Manifest files consist of the following sections:

- [`name`] — Extension name
- [`description`] - Description of the extension
- [`entry_point`] — Execution entry point
- [`[permissions]`][perms] — Permissions required for execution
  - [`read`][read] — Required read path permissions
  - [`write`][write] — Required write path permissions
  - [`env`][env] — Required environment variable permissions
  - [`run`][run] — Required process execution permissions
  - [`net`][net] — Required network domain permissions

[TOML]: https://toml.io
[`name`]: https://docs.phylum.io/docs/extension_manifest#name
[`description`]: https://docs.phylum.io/docs/extension_manifest#description
[`entry_point`]: https://docs.phylum.io/docs/extension_manifest#entry-point
[perms]: https://docs.phylum.io/docs/extension_manifest#permissions
[read]: https://docs.phylum.io/docs/extension_manifest#read
[write]: https://docs.phylum.io/docs/extension_manifest#write
[env]: https://docs.phylum.io/docs/extension_manifest#env
[run]: https://docs.phylum.io/docs/extension_manifest#run
[net]: https://docs.phylum.io/docs/extension_manifest#net

## Name

The extension name is used as the subcommand for executing the extension and
acts as an identifier when referring to it.

The name is required and must use only lowercase alphanumeric characters,
hyphens (`-`), or underscores (`_`).

```toml
name = "hello-world_1"
```

## Description

The description is an optional short blurb about the extension. This should be
plain text (not Markdown).

```toml
description = "Example extension that greets the world"
```

## Entry Point

The entry point points to the file which should be loaded as the initial
JavaScript module. Generally this will be the `main.ts` file in the extension
root directory.

Phylum CLI extensions support both JavaScript and TypeScript out of the box,
transpiling TypeScript automatically before execution.

```toml
entry_point = "main.ts"
```

## Permissions

Since extensions are executed inside Deno's JavaScript sandbox, no external
effects can be performed without requesting the necessary permissions.

Users will be prompted to agree to these permissions during install, they are
later validated during execution relative to the active working directory when
running it.

`[permissions]` is an optional table of key-value pairs where each key is a type
of permission.

### Read

Read permissions list file paths which can be read from by the extension.

Granting permissions to a directory will also allow the extension to access any
child directories and files inside them.

This is an optional key-value pair where the value is either a boolean, or an
array containing the allowed directories.

```toml
[permissions]
# ...
read = [
    "./path/to/file.txt",
    "./path/to/directory",
    "./config_file.yaml",
]
```

```toml
[permissions]
# ...
read = true
```

### Write

Write permissions list file paths which can be written to by the extension.

Granting permissions to a directory will also allow the extension to access any
child directories and files inside them.

This is an optional key-value pair where the value is either a boolean, or an
array containing the allowed directories.

```toml
[permissions]
# ...
write = ["./output_file.txt"]
```

```toml
[permissions]
# ...
write = true
```

### Env

Env permissions list environment variables which can be read by the extension.

This is an optional key-value pair where the value is either a boolean, or an
array containing the allowed environment variables.

```toml
[permissions]
# ...
env = ["PHYLUM_API_KEY"]
```

```toml
[permissions]
# ...
env = true
```

### Run

Run permissions list executable paths which can be executed by the extension.

The executable paths take `$PATH` into account, so it is recommended to avoid
using absolute paths to improve portability.

The paths also need to match **exactly** with the process executed by the
extension. `/usr/bin/curl` cannot be executed when `curl` was requested as
permission and vice versa.

This is an optional key-value pair where the value is either a boolean, or an
array containing the allowed executables.

```toml
[permissions]
# ...
run = ["npm", "yarn"]
```

```toml
[permissions]
# ...
run = true
```

### Net

Net permissions list domains which can be accessed by the extension.

Network permissions only describe the domains and subdomains an extension has
access to, regardless of its path segments or protocol scheme. Access to a
domain does not automatically grant access to all of its subdomains.

If the requested domain requests a redirect, you'll also require permissions to
access the redirect target. It's easiest to just specify the redirect target
directly when making requests, otherwise you'll have to request permissions for
both domains.

This is an optional key-value pair where the value is either a boolean, or an
array containing the allowed domains.

```toml
[permissions]
# ...
net = ["www.phylum.io", "phylum.io"]
```

```toml
[permissions]
# ...
net = true
```
