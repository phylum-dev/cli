# phylum project link

Link a repository to a project

```sh
Usage: phylum project link [OPTIONS] <NAME>
```

## Arguments

`<NAME>`
&emsp; Name of the project

## Options

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group owning the project

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

## Examples

```sh
# Link current folder to an existing project named 'sample'
$ phylum project link sample

# Link current folder to an existing project named 'sample' owned by the group 'sGroup'
$ phylum project link -g sGroup sample
```
