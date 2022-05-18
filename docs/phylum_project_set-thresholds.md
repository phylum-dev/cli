---
title: phylum project set-thresholds
category: 6255e67693d5200013b1fa3e
hidden: false
---
Interactively set risk domain thresholds for a project
```sh
phylum project set-thresholds <name> [OPTIONS]
```
Resulting scores at or below the defined threshold will fail.

### Options
`-g`, `--group <group_name>`
&emsp; Group owning the project

### Examples
```sh
# Interactively set risk domain thresholds for the 'sample' project
$ phylum project set-thresholds sample

# Interactively set risk domain thresholds for the 'sample' project owned by the 'sGroup' group
$ phylum project set-thresholds sample -g sGroup
```
