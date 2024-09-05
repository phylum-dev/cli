# phylum group delete

Delete a group

```sh
Usage: phylum group delete [OPTIONS] <GROUP_NAME>
```

## Arguments

`<GROUP_NAME>`
&emsp; Name for the group to be deleted

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
# Delete an existing group named `sample`
$ phylum group delete sample

# Delete the group `sample` from the `test` organization
$ phylum group delete --org test sample

# Make `test` the default organization for all operations,
# then delete the group `sample` from it.
$ phylum org link test
$ phylum group delete sample
```
