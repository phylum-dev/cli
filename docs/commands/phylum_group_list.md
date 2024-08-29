# phylum group list

List all groups the user is a member of

```sh
Usage: phylum group list [OPTIONS]
```

## Options

`-j`, `--json`
&emsp; Produce output in json format (default: false)

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
# List all groups the user is a member of
$ phylum group list

# List all groups the user is a member of with json output
$ phylum group list --json
```
