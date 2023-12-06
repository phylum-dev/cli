{PH-HEADER}

{PH-MARKDOWN}

### Examples

```sh
# Parse a dependency file
$ phylum parse package-lock.json

# Parse the `Cargo.lock` and `lockfile` files as cargo dependency files
$ phylum parse --type cargo Cargo.lock lockfile
```
