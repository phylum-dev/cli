---
title: phylum project set-thresholds
category: 6255e67693d5200013b1fa3e
hidden: false
---

Interactively set risk domain thresholds for a project

```sh
Usage: phylum project set-thresholds [OPTIONS] <name>
```

### Arguments

<name>
&emsp; Name of the project

### Options

-g, --group <group_name>
&emsp; Group owning the project

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Examples

```sh
# Interactively set risk domain thresholds for the 'sample' project
$ phylum project set-thresholds sample

# Interactively set risk domain thresholds for the 'sample' project owned by the 'sGroup' group
$ phylum project set-thresholds -g sGroup sample
```
