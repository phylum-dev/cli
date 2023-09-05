---
title: phylum parse
---

Parse lockfiles and output their packages as JSON

```sh
Usage: phylum parse [OPTIONS] [LOCKFILE]...
```

### Arguments

`[LOCKFILE]`
&emsp; The package lockfiles to submit

### Options

`-t`, `--lockfile-type` `<type>`
&emsp; Lockfile type used for all lockfiles (default: auto)
&emsp; Accepted values: `npm`, `yarn`, `pnpm`, `gem`, `pip`, `poetry`, `pipenv`, `mvn`, `gradle`, `nugetlock`, `msbuild`, `go`, `cargo`, `spdx`, `cyclonedx`, `auto`

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Examples

```sh
# Parse a lockfile
$ phylum parse package-lock.json

# Parse the `Cargo.lock` and `lockfile` files as cargo lockfiles
$ phylum parse --lockfile-type cargo Cargo.lock lockfile
```
