---
title: phylum package
category: 6255e67693d5200013b1fa3e
hidden: false
---

Retrieve the details of a specific package
```sh
phylum package [OPTIONS] <name> <version>
```

### Options

`--filter`
&emsp; Provide a filter used to limit the issues displayed

EXAMPLES
# Show only issues with severity of at least 'high'
    --filter=high

# Show issues with severity of 'critical' in the 'author'
and 'engineering' domains
    --filter=crit,aut,eng

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-t`, `--package-type <type>`
&emsp; The type of package: `npm`, `pypi`, `nuget`, `maven`, `rubygems`

### Examples

```sh
# Query specific package details
$ phylum package -t npm axios 0.19.0
```
