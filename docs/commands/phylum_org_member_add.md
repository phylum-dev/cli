# phylum org member add

Add user to organization

```sh
Usage: phylum org member add [OPTIONS] <USER>...
```

## Arguments

`<USER>`
&emsp; User(s) to be added

## Options

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
# Add user `demo@phylum.io` to the `sample` organization
$ phylum org -o sample member add demo@phylum.io
```
