{PH-HEADER}

{PH-MARKDOWN}

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
