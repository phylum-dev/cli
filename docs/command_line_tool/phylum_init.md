---
title: phylum init
category: 6255e67693d5200013b1fa3e
hidden: false
---

Setup a new Phylum project

```sh
phylum init [OPTIONS] [PROJECT_NAME]
```

### Arguments

`[PROJECT_NAME]`
&emsp; Phylum project name

### Options

`-g`, `--group <GROUP_NAME>`
&emsp; Group which will be the owner of the project

`-l`, `--lockfile <LOCKFILE>`
&emsp; Project lockfile name

`-t`, `--lockfile-type <LOCKFILE_TYPE>`
&emsp; Project lockfile type

`-f`, `--force`
&emsp; Overwrite existing configurations without confirmation

### Examples

```sh
# Interactively initialize the Phylum project.
$ phylum init

# Create the `demo` project with a yarn lockfile and no associated group.
$ phylum init --lockfile yarn.lock --lockfile-type yarn demo
```
