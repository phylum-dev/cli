---
title: phylum package
category: 6255e67693d5200013b1fa3e
hidden: false
---

Retrieve the details of a specific package

```sh
Usage: phylum package [OPTIONS] <name> <version>
```

### Arguments

<name>
&emsp; The name of the package.

<version>
&emsp; The version of the package.

### Options

-t, --package-type <type>
&emsp; The type of the package ("npm", "rubygems", "pypi", "maven", "nuget", "golang", "cargo")

-j, --json
&emsp; Produce output in json format (default: false)

-f, --filter <filter>
&emsp; Provide a filter used to limit the issues displayed

&nbsp;&nbsp;&nbsp;&nbsp;EXAMPLES:
&nbsp;&nbsp;&nbsp;&nbsp;\# Show only issues with severity of at least 'high'
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=high

&nbsp;&nbsp;&nbsp;&nbsp;\# Show issues with severity of 'critical' in the 'author'
&nbsp;&nbsp;&nbsp;&nbsp;and 'engineering' domains
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=crit,aut,eng

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Details

If the requested package has not yet been analyzed by Phylum, it will
automatically be submitted for [processing].

[processing]: https://docs.phylum.io/docs/processing

The following order is used to determine which lockfile will be parsed:
 - CLI `--lockfile` parameters
 - Lockfiles in the `.phylum_project` file specified during `phylum init`
 - Recursive filesystem search

If any of these locations provides a lockfile, no further search will be done.
Recursive filesystem search takes common ignore files like `.gitignore` and
`.ignore` into account.

### Examples

```sh
# Query specific package details
$ phylum package -t npm axios 0.19.0
```
