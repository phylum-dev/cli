---
title: phylum project create
category: 6255e67693d5200013b1fa3e
parentDoc: 62757a105ec2660021a19e4d
hidden: false
---

Create a new project

```sh
Usage: phylum project create [OPTIONS] <name>
```

### Arguments

`<name>`
&emsp; Name of the project

### Options

`-g`, `--group` `<group_name>`
&emsp; Group which will be the owner of the project

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Examples

```sh
# Create a new project named 'sample'
$ phylum project create sample

# Create a new project named 'sample' owned by the group 'sGroup'
$ phylum project create -g sGroup sample
```
