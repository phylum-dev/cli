# phylum project create

Create a new project

```sh
Usage: phylum project create [OPTIONS] <NAME>
```

## Arguments

`<NAME>`
&emsp; Name of the project

## Options

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group which will be the owner of the project

`-r`, `--repository-url` `<repository_url>`
&emsp; Repository URL of the project

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
# Create a new project named 'sample'
$ phylum project create sample

# Create a new project named 'sample' owned by the group 'sGroup'
$ phylum project create -g sGroup sample
```
