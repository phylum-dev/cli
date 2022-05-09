---
title: phylum history
category: 6255e67693d5200013b1fa3e
hidden: false
---

Return information about historical scans
```sh
phylum history [OPTIONS] [JOB_ID]
```
`<JOB_ID>`
&emsp; The job id to query, or `current` for the most recent job

### Options
`--filter <filter>`
&emsp; Provide a filter used to limit the issues displayed

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-p`, `--project <project_name>`
&emsp; Project name used to filter jobs

`-v`, `--verbose`
&emsp; Increase verbosity of API response

### Examples
```sh
# List the last 30 analysis runs
$ phylum history

#View the analysis results of a historical job
$ phylum history 338ea79f-0e82-4422-9769-4e583a84599f

#View the analysis results of the most recent job
$ phylum history current

# View a list of analysis runs for the 'sample' project
$ phylum history --project sample
```
