---
title: phylum project
category: 6255e67693d5200013b1fa3e
hidden: false
---

Create, list, link and set thresholds for projects

```sh
phylum project [OPTIONS] [SUBCOMMAND]
```

### Options
`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-g`, `--group <group_name>`
&emsp; Group to list projects for

### Commands
* [phylum project create](https://docs.phylum.io/docs/phylum_project_create)
* [phylum project link](https://docs.phylum.io/docs/phylum_project_link)
* [phylum project list](https://docs.phylum.io/docs/phylum_project_list)
* [phylum project set-thresholds](https://docs.phylum.io/docs/phylum_project_set-thresholds)

### Examples
```sh
# List all projects for the current account
$ phylum project

# List all projects for the 'sample' group
$ phylum project -g sample

# Return json response of all projects for the current account
$ phylum project --json
```
