---
title: phylum project list
category: 6255e67693d5200013b1fa3e
hidden: false
---
List all existing projects
```sh
phylum project list [OPTIONS]
```

### Options
`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-g`, `--group <group_name>`
&emsp; Group to list projects for

### Examples
```sh
# List all existing projects
$ phylum project list

# List all existing projects with json output
$ phylum project list --json

# List all existing projects for the 'sample' group
$ phylum project list -g sample
```
