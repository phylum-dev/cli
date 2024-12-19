# phylum firewall log

Show firewall activity log

```sh
Usage: phylum firewall log [OPTIONS] <GROUP_NAME>
```

## Arguments

`<GROUP_NAME>`
&emsp; Firewall group to list log activity for

## Options

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`--package-type` `<PACKAGE_TYPE>`
&emsp; Only show logs matching this package type
&emsp; Accepted values: `npm`, `gem`, `pypi`, `maven`, `nuget`, `cargo`

`--purl` `<PURL>`
&emsp; Only show logs matching this PURL

`--action` `<ACTION>`
&emsp; Only show logs matching this log action
&emsp; Accepted values: `Download`, `AnalysisSuccess`, `AnalysisFailure`, `AnalysisWarning`

`--before` `<TIMESTAMP>`
&emsp; Only show logs created before this timestamp (RFC3339 format)

`--after` `<TIMESTAMP>`
&emsp; Only show logs created after this timestamp (RFC3339 format)

`--limit` `<COUNT>`
&emsp; Maximum number of log entries to show

`-o`, `--org` `<ORG>`
&emsp; Phylum organization

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

## Examples

```sh
# Show logs for packages which failed analysis for the group `demo`.
$ phylum firewall log demo --action AnalysisFailure

# Show logs which were created after 2024 for the group `demo`.
$ phylum firewall log demo --after 2024-01-01T00:00:0.0Z

# Show logs for libc regardless of its version for the group `demo`.
$ phylum firewall log demo --package pkg:cargo/libc
```
