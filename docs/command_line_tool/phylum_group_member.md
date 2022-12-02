---
title: phylum group member
category: 6255e67693d5200013b1fa3e
parentDoc: 62866c2ce78584036d7cbbf7
hidden: false
---

Manage group members

```sh
Usage: phylum group member [OPTIONS] --group <GROUP> [COMMAND]
```

### Options

-g, --group <GROUP>
&emsp; Group to list the members for

-j, --json
&emsp; Produce member list in json format (default: false)

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Commands

* [phylum group member list](./phylum_group_member_list)
* [phylum group member add](./phylum_group_member_add)
* [phylum group member remove](./phylum_group_member_remove)

### Examples

```sh
# List all group members for the 'sample' group
$ phylum group member --group sample
```
