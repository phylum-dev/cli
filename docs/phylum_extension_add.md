---
title: phylum extension add
category: 6255e67693d5200013b1fa3e
hidden: true
---
To install an extension, run the following command:

```sh
phylum extension add path/to/extension
```

The extension will be installed under `$XDG_DATA_HOME/phylum/extensions/<ext_name>`.
Once installed, the extension will be accessible via the Phylum CLI:

```sh
phylum <ext_name> [arguments...]
```
