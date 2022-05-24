---
title: Extension format
category: 6255e67693d5200013b1fa41
hidden: true
---
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
