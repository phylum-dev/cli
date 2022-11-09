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

`-g`, `--group <group_name>`
&emsp; Group which will be the owner of the project

### Examples

```sh
# Interactively initialize the Phylum project.
$ phylum init

# Create the `demo` project without interactivity or associated group.
$ phylum init demo

# Create the `demo` project without interactivity for the group `users`.
$ phylum init demo --group users
```
