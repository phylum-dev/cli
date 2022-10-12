---
title: phylum parse
category: 6255e67693d5200013b1fa3e
hidden: false
---
Parse a lockfile and output the packages as JSON
```sh
phylum parse [OPTIONS] <LOCKFILE>
```

### Options
`-t`, `--lockfile-type`
&emsp; The type of the lockfile (default: `auto`): `yarn`, `npm`, `gem`, `pip`, `pipenv`, `poetry`, `mvn`, `gradle`, `nuget`, `go`, `rust`, `auto`

### Examples
```sh
# Parse a lockfile
$ phylum parse -t npm package-lock.json
```
