---
title: Extensions
category: 61e72e3a50a88e001a92ee5d
---

It is possible to extend the Phylum CLI with external sub-commands.

To install an extension, run the following command:

```sh
phylum extension add path/to/extension
```

The extension will be installed under `$XDG_DATA_HOME/phylum/extensions/<ext_name>`.

Once installed, the extension will be accessible via the Phylum CLI:

```sh
phylum <ext_name> [arguments...]
```

To list the currently installed extension, run the following command:

```sh
phylum extension list
```

To uninstall a previously installed extension, run the following command:

```sh
phylum extension remove <ext_name>
```

## Extension format

**TODO**: Rectify this section once the final decisions on the format have been taken.

An extension is comprised of a directory containing a *manifest file*, named
`PhylumExt.toml`, an executable file (the *entry point*) and as many auxilliary
files as the extension may require. The `PhylumExt.toml` manifest is structured
this way:

```toml
name = "extension-name"
description = "Brief description of the extension"
entry_point = "sample_extension.sh"
```

The *entry point* is the executable that will be run via `phylum extension-name`.
All arguments passed via the CLI will be forwarded to the extension executable.
