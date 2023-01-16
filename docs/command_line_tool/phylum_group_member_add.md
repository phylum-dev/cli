---
title: phylum group member add
category: 6255e67693d5200013b1fa3e
parentDoc: 62866c2ce78584036d7cbbf7
hidden: false
---

Add user to group

```sh
Usage: phylum group member --group <GROUP> add [OPTIONS] <USER>...
```

### Arguments

<USER>
&emsp; User(s) to be added

### Options

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Examples

```sh
# Add user `demo@phylum.io` to the `sample` group
$ phylum group member --group sample add demo@phylum.io
```
