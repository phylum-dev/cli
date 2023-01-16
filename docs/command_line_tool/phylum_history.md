---
title: phylum history
category: 6255e67693d5200013b1fa3e
hidden: false
---

Return information about historical jobs

```sh
Usage: phylum history [OPTIONS] [JOB_ID]
```

### Arguments

[JOB_ID]
&emsp; The job id to query

### Options

-f, --filter <filter>
&emsp; Provide a filter used to limit the issues displayed

&nbsp;&nbsp;&nbsp;&nbsp;EXAMPLES:
&nbsp;&nbsp;&nbsp;&nbsp;\# Show only issues with severity of at least 'high'
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=high

&nbsp;&nbsp;&nbsp;&nbsp;\# Show issues with severity of 'critical' in the 'author'
&nbsp;&nbsp;&nbsp;&nbsp;and 'engineering' domains
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=crit,aut,eng

-j, --json
&emsp; Produce output in json format (default: false)

-p, --project <project_name>
&emsp; Project name used to filter jobs

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Examples

```sh
# List the last 30 analysis runs
$ phylum history

# View the analysis results of a historical job
$ phylum history 338ea79f-0e82-4422-9769-4e583a84599f

# View a list of analysis runs for the 'sample' project
$ phylum history --project sample
```
