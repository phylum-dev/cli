---
title: phylum project link
category: 6255e67693d5200013b1fa3e
hidden: false
---
Link a repository to a project
```sh
phylum project link [OPTIONS] <name>
```
This command will create the appropriate `.phylum_project` file in the current working directory.

### Options
`-g`, `--group <group_name>`
&emsp; Group owning the project

### Examples
```sh
# Link current folder to an existing project named 'sample'
$ phylum project link sample

# Link current folder to an existing project named 'sample' owned by the group 'sGroup'
$ phylum project link -g sGroup sample
```
