---
title: phylum project link
category: 6255e67693d5200013b1fa3e
parentDoc: 62757a105ec2660021a19e4d
hidden: false
---

Link a repository to a project

```sh
Usage: phylum project link [OPTIONS] <name>
```

### Arguments

`<name>`
&emsp; Name of the project

### Options

`-g`, `--group` `<group_name>`
&emsp; Group owning the project

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Examples

```sh
# Link current folder to an existing project named 'sample'
$ phylum project link sample

# Link current folder to an existing project named 'sample' owned by the group 'sGroup'
$ phylum project link -g sGroup sample
```
