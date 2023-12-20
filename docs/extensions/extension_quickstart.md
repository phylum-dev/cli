# Extension Quickstart

## Creating an extension

The best way to get started with writing your own Phylum CLI extension, is to
generate an extension skeleton using the `phylum extension new` subcommand.
We'll use `my-extension` as an example in this guide:

```sh
phylum extension new my-extension
```

Once finished, we should find a new directory called `my-extension` in our
current working directory, containing the files `main.ts` and `PhylumExt.toml`.

## Extension structure

Extensions always contain at least two files, the manifest describing the
extension (`PhylumExt.toml`), and the entrypoint where the extension's execution
will begin. Any additional source files can be included in the extension
directory and imported from the entrypoint.

The manifest file contains metadata about the extension beyond its executable
source code. All available options can be found in [the manifest format].

[the manifest format]: ./extension_manifest.md

## Installation

Since the generated extension skeleton is a fully functional extension, we can
go ahead and install it right away:

```sh
phylum extension install ./my-extension
```

## Execution

Once successfully installed, our extension can be executed by using its name as
a subcommand for the phylum CLI:

```shellsession
$ phylum my-extension
Hello, World!
```

The `Hello, World!` message confirms that our extension is working correctly.
