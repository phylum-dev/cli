---
title: phylum parse
category: 6255e67693d5200013b1fa3e
hidden: false
---

Parse dependency files and output their packages as JSON

```sh
Usage: phylum parse [OPTIONS] [DEPENDENCY_FILE]...
```

### Arguments

`[DEPENDENCY_FILE]`
&emsp; Path to the dependency file to parse

### Options

`-t`, `--type` `<TYPE>`
&emsp; Dependency file type used for all lockfiles (default: auto)
&emsp; Accepted values: `npm`, `yarn`, `pnpm`, `gem`, `pip`, `poetry`, `pipenv`, `mvn`, `gradle`, `nugetlock`, `msbuild`, `go`, `cargo`, `spdx`, `cyclonedx`, `auto`

`--skip-sandbox`
&emsp; Run lockfile generation without sandbox protection

`--no-generation`
&emsp; Disable generation of lockfiles from manifests

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Details

The following order is used to determine which dependency file will be parsed:

- CLI `DEPENDENCY_FILE` argument
- Dependency files in the `.phylum_project` file specified during `phylum init`
- Recursive filesystem search

If any of these locations provides a dependency file, no further search will be
done. Recursive filesystem search takes common ignore files like `.gitignore`
and `.ignore` into account.

### Examples

```sh
# Parse a dependency file
$ phylum parse package-lock.json

# Parse the `Cargo.lock` and `lockfile` files as cargo dependency files
$ phylum parse --type cargo Cargo.lock lockfile
```
