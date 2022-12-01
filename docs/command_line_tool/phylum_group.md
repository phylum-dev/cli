---
title: phylum group
category: 6255e67693d5200013b1fa3e
hidden: false
---

Interact with user groups

```sh
Usage: phylum group [OPTIONS] [COMMAND]
```

### Options

-j, --json
&emsp; Produce group list in json format (default: false)

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Commands

* [phylum group list](./phylum_group_list)
* [phylum group create](./phylum_group_create)
* [phylum group member](./phylum_group_member)
* [phylum group transfer](./phylum_group_transfer)

### Examples

```sh
# List all groups for the current account
$ phylum group

# Return json response of all groups for the current account
$ phylum group --json
```
