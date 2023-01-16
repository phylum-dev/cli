---
title: phylum parse
category: 6255e67693d5200013b1fa3e
hidden: false
---

Parse a lockfile and output its packages as JSON

```sh
Usage: phylum parse [OPTIONS] [LOCKFILE]
```

### Arguments

[LOCKFILE]
&emsp; The package lock file to submit.

### Options

-t, --lockfile-type <type>
&emsp; The type of the lock file (default: auto)
&emsp; Accepted values: `yarn`, `npm`, `gem`, `pip`, `pipenv`, `poetry`, `mvn`, `gradle`, `nuget`, `go`, `cargo`, `auto`

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Examples

```sh
# Parse a lockfile
$ phylum parse -t npm package-lock.json
```
