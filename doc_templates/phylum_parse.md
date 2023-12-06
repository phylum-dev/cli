{PH-HEADER}

{PH-MARKDOWN}

### Details

The following order is used to determine which lockfile will be parsed:

- CLI `DEPENDENCY_FILE` argument
- Lockfiles in the `.phylum_project` file specified during `phylum init`
- Recursive filesystem search

If any of these locations provides a lockfile, no further search will be done.
Recursive filesystem search takes common ignore files like `.gitignore` and
`.ignore` into account.

### Examples

```sh
# Parse a dependency file
$ phylum parse package-lock.json

# Parse the `Cargo.lock` and `lockfile` files as cargo dependency files
$ phylum parse --type cargo Cargo.lock lockfile
```
