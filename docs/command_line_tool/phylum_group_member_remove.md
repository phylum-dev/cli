---
title: phylum group member remove
category: 6255e67693d5200013b1fa3e
parentDoc: 62866c2ce78584036d7cbbf7
hidden: false
---

Remove user from group

```sh
Usage: phylum group member --group <GROUP> remove [OPTIONS] <USER>...
```

### Arguments

<USER>
&emsp; User(s) to be removed

### Options

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Examples

```sh
# Remove user `demo@phylum.io` from the `sample` group
$ phylum group member --group remove demo@phylum.io
```
