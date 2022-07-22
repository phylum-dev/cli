---
title: phylum extension run
category: 6255e67693d5200013b1fa3e
hidden: true
---

Run an extension from a directory

```sh
phylum extension run [OPTIONS] <PATH> [OPTIONS]...
```

### Options

`-y`, `--yes`
&emsp; Automatically accept requested permissions

### Details

The extension will be run without prior installation.

The first set of options are for the `run` command. The second set of options
are for the extension.

### Examples

```sh
phylum extension run --yes ./my-extension --help
```
