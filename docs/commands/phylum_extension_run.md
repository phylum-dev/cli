---
title: phylum extension run
category: 6255e67693d5200013b1fa3e
parentDoc: 62d04d1ec90dcf008e476330
hidden: false
---

Run an extension from a directory

```sh
Usage: phylum extension run [OPTIONS] <PATH> [OPTIONS]...
```

### Arguments

`<PATH>`

`[OPTIONS]`
&emsp; Extension parameters

### Options

`-y`, `--yes`
&emsp; Automatically accept requested permissions

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

### Details

The extension will be run without prior installation.

The first set of options are for the `run` command. The second set of options
are for the extension.

### Examples

```sh
phylum extension run --yes ./my-extension --help
```
