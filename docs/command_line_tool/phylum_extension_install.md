---
title: phylum extension install
category: 6255e67693d5200013b1fa3e
hidden: false
---

Install an extension

```sh
phylum extension install [OPTIONS] <PATH>
```

### Options
`--accept-permissions`
&emsp; Automatically accept requested permissions

`--overwrite`
&emsp; Overwrite existing extension

`-y`, `--yes`
&emsp; Accept permissions and overwrite existing (same as `--overwrite --accept-permissions`)

### Details

The extension will be installed under `$XDG_DATA_HOME/phylum/extensions/<EXT_NAME>`.
If `$XDG_DATA_HOME` is not set, it will default to `$HOME/.local/share/phylum/extensions/<EXT_NAME>`.

Once installed, the extension will be accessible via the Phylum CLI:

```sh
phylum <EXT_NAME> [OPTIONS]...
```
