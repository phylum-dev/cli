# phylum group create

Create a new group

```sh
Usage: phylum group create [OPTIONS] <GROUP_NAME>
```

## Arguments

`<GROUP_NAME>`
&emsp; Name for the new group

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
# Create a new group named `sample`
$ phylum group create sample

# Create a group `sample` under the `test` organization
$ phylum group create --org test sample

# Make `test` the default organization for all operations,
# then create a new group `sample` under it.
$ phylum org link test
$ phylum group create sample
```
