---
title: phylum project
category: 6255e67693d5200013b1fa3e
hidden: false
---

Create, list, link and set thresholds for projects

```sh
Usage: phylum project [OPTIONS] [COMMAND]
```

### Options

-j, --json
&emsp; Produce output in json format (default: false)

-g, --group <group_name>
&emsp; Group to list projects for

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Commands

* [phylum project create](./phylum_project_create)
* [phylum project delete](./phylum_project_delete)
* [phylum project list](./phylum_project_list)
* [phylum project link](./phylum_project_link)
* [phylum project set-thresholds](./phylum_project_set-thresholds)

### Examples

```sh
# List all projects for the current account
$ phylum project

# List all projects for the 'sample' group
$ phylum project -g sample

# Return json response of all projects for the current account
$ phylum project --json
```
