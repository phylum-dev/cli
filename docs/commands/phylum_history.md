# phylum history

Return information about historical jobs

```sh
Usage: phylum history [OPTIONS] [JOB_ID]
```

## Arguments

`[JOB_ID]`
&emsp; The job id to query

## Options

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-p`, `--project` `<PROJECT_NAME>`
&emsp; Project to be queried

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group to be queried

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
# List the last 30 analysis runs
$ phylum history

# View the analysis results of a historical job
$ phylum history 338ea79f-0e82-4422-9769-4e583a84599f

# View a list of analysis runs for the 'sample' project
$ phylum history --project sample
```
