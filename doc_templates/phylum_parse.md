{PH-HEADER}

{PH-MARKDOWN}

### Examples

```sh
# Parse a lockfile
$ phylum parse package-lock.json

# Parse the `Cargo.lock` and `lockfile` files as cargo lockfiles
$ phylum parse --lockfile-type cargo Cargo.lock lockfile
```
