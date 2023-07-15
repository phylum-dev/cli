{PH-HEADER}

{PH-MARKDOWN}

### Details

If the requested package has not yet been analyzed by Phylum, it will
automatically be submitted for [processing].

[processing]: https://docs.phylum.io/docs/processing

The following order is used to determine which lockfile will be parsed:

- CLI `--lockfile` parameters
- Lockfiles in the `.phylum_project` file specified during `phylum init`
- Recursive filesystem search

If any of these locations provides a lockfile, no further search will be done.
Recursive filesystem search takes common ignore files like `.gitignore` and
`.ignore` into account.

### Examples

```sh
# Query specific package details
$ phylum package -t npm axios 0.19.0
```
