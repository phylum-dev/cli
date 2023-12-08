---
title: phylum package
category: 6255e67693d5200013b1fa3e
hidden: false
---

Retrieve the details of a specific package

```sh
Usage: phylum package [OPTIONS] <TYPE> <NAME> <VERSION>
```

### Arguments

`<TYPE>`
&emsp; Package ecosystem type
&emsp; Accepted values: `npm`, `rubygems`, `pypi`, `maven`, `nuget`, `golang`, `cargo`

`<NAME>`
&emsp; The name of the package.

`<VERSION>`
&emsp; The version of the package.

### Options

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-f`, `--filter` `<FILTER>`
&emsp; Provide a filter used to limit the issues displayed

&nbsp;&nbsp;&nbsp;&nbsp;EXAMPLES:
&nbsp;&nbsp;&nbsp;&nbsp;\# Show only issues with severity of at least 'high'
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=high

&nbsp;&nbsp;&nbsp;&nbsp;\# Show issues with severity of 'critical' in the 'author'
&nbsp;&nbsp;&nbsp;&nbsp;and 'engineering' domains
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=crit,aut,eng

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Details

If the requested package has not yet been analyzed by Phylum, it will
automatically be submitted for [processing].

[processing]: https://docs.phylum.io/docs/processing

### Examples

```sh
# Query specific package details
$ phylum package -t npm axios 0.19.0
```
