{PH-HEADER}

{PH-MARKDOWN}

## Examples

```sh
# Interactively initialize the Phylum project.
$ phylum init

# Create the `demo` project with a yarn lockfile and no associated group.
$ phylum init --dependency-file yarn.lock --type yarn demo

# Create the `demo` project in the `sample` group of the `test` organization.
$ phylum init --org test --group sample demo
```
