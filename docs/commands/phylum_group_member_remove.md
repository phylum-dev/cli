# phylum group member remove

Remove user from group

```sh
Usage: phylum group member --group <GROUP> remove [OPTIONS] <USER>...
```

## Arguments

`<USER>`
&emsp; User(s) to be removed

## Options

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

## Examples

```sh
# Remove user `demo@phylum.io` from the `sample` group
$ phylum group member --group sample remove demo@phylum.io
```
