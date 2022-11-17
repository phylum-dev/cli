---
title: phylum group member
category: 6255e67693d5200013b1fa3e
hidden: false
---

Manage group members

```sh
phylum group member [OPTIONS] --group <GROUP> [COMMAND]
```

### Options

`-j`, `--json`
&emsp; Produce member list in json format (default: false)

`-g`, `--group <GROUP>`
&emsp; Group to list the members for

### Commands
* [phylum group_member_list](https://docs.phylum.io/docs/phylum_group_member_list)
* [phylum group_member_add](https://docs.phylum.io/docs/phylum_group_member_add)
* [phylum group_member_remove](https://docs.phylum.io/docs/phylum_group_member_remove)

### Examples

```sh
# List all group members for the 'sample' group
$ phylum group member --group sample
```
