---
title: phylum project create
category: 6255e67693d5200013b1fa3e
hidden: false
---
Create a new project
```sh
phylum project create [OPTIONS] <name>
```
This command will create the appropriate `.phylum_project` file in the current working directory.

### Options
`-g`, `--group <group_name>`
&emsp; Group which will be the owner of the project

### Examples
```sh
# Create a new project named 'sample'
$ phylum project create sample

# Create a new project named 'sample' owned by the group 'sGroup'
$ phylum project create -g sGroup sample
```
