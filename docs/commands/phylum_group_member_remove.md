# phylum group member remove

Remove user from group

```sh
Usage: phylum group member --group <GROUP> remove [OPTIONS] <USER>...
```

## Arguments

`<USER>`
&emsp; User(s) to be removed

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
# Remove user `demo@veracode.com` from the `sample` group
$ phylum group member --group sample remove demo@veracode.com
```
