# phylum org list

List all organizations the user is a member of

```sh
Usage: phylum org list [OPTIONS]
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
# List all organizations the user is a member of
$ phylum org list

# List all organizations the user is a member of with json output
$ phylum org list --json
```
