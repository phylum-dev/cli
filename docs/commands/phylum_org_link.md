# phylum org link

Select an organization as default for all operations

```sh
Usage: phylum org link [OPTIONS] [ORG]
```

## Arguments

`[ORG]`
&emsp; Organization to use as default

## Options

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

## Examples

```sh
# Interactively select an organization for all future operations
$ phylum org link

# Set `sample` as the default organization for all future operations
$ phylum org link sample
```
