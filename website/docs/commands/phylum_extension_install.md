---
title: phylum extension install
---

Install extension

```sh
Usage: phylum extension install [OPTIONS] <PATH>
```

### Arguments

`<PATH>`

### Options

`-y`, `--yes`
&emsp; Accept permissions and overwrite existing extensions (same as \`--overwrite --accept-permissions\`)

`--accept-permissions`
&emsp; Automatically accept requested permissions

`--overwrite`
&emsp; Overwrite existing extension

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Details

The extension will be installed under `$XDG_DATA_HOME/phylum/extensions/<EXT_NAME>`. If `$XDG_DATA_HOME` is not set, it will default to `$HOME/.local/share/phylum/extensions/<EXT_NAME>`.

Once installed, the extension will be accessible via the Phylum CLI:

```sh
phylum <EXT_NAME> [OPTIONS]...
```
