# phylum project list

List all existing projects

```sh
Usage: phylum project list [OPTIONS]
```

## Options

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group to list projects for

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
# List all existing projects
$ phylum project list

# List all existing projects with json output
$ phylum project list --json

# List all existing projects for the `sample` group
$ phylum project list -g sample
```
