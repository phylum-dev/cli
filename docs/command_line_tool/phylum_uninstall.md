---
title: phylum uninstall
category: 6255e67693d5200013b1fa3e
hidden: false
---

Uninstall the Phylum CLI

```sh
Usage: phylum uninstall [OPTIONS]
```

### Options

-p, --purge
&emsp; Remove all files, including configuration files (default: false)

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Examples

```sh
# Remove installed phylum binary and data files
$ phylum uninstall

# Remove all installed phylum files
$ phylum uninstall --purge
```
