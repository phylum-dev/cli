{PH-HEADER}

{PH-MARKDOWN}

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
