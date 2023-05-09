---
title: phylum init
category: 6255e67693d5200013b1fa3e
hidden: false
---

Setup a new Phylum project

```sh
Usage: phylum init [OPTIONS] [PROJECT_NAME]
```

### Arguments

[PROJECT_NAME]
&emsp; Phylum project name

### Options

-g, --group <GROUP_NAME>
&emsp; Group which will be the owner of the project

-l, --lockfile <LOCKFILE>
&emsp; Project-relative lock file path

-t, --lockfile-type <type>
&emsp; Lock file type used for all lock files (default: auto)
&emsp; Accepted values: `npm`, `yarn`, `gem`, `poetry`, `pip`, `pipenv`, `mvn`, `gradle`, `nuget`, `go`, `cargo`, `spdx`, `auto`

-f, --force
&emsp; Overwrite existing configurations without confirmation

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Examples

```sh
# Interactively initialize the Phylum project.
$ phylum init

# Create the `demo` project with a yarn lockfile and no associated group.
$ phylum init --lockfile yarn.lock --lockfile-type yarn demo
```
