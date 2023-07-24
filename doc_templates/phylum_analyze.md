{PH-HEADER}

{PH-MARKDOWN}

### Details

The following order is used to determine which lockfile will be analyzed:
 - CLI `--lockfile` parameters
 - Lockfiles in the `.phylum_project` file specified during `phylum init`
 - Recursive filesystem search

If any of these locations provides a lockfile, no further search will be done.
Recursive filesystem search takes common ignore files like `.gitignore` and
`.ignore` into account.

### Examples

```sh
# Analyze your project's default lockfile
$ phylum analyze

# Analyze a Maven lock file with a verbose json response
$ phylum analyze --json --verbose effective-pom.xml

# Analyze a PyPI lock file and apply a label
$ phylum analyze --label test_branch requirements.txt

# Analyze a Poetry lock file and return the results to the 'sample' project
$ phylum analyze -p sample poetry.lock

# Analyze a NuGet lock file using the 'sample' project and 'sGroup' group
$ phylum analyze -p sample -g sGroup packages.lock.json

# Analyze a RubyGems lock file and return a verbose response with only critical malware
$ phylum analyze --verbose --filter=crit,mal Gemfile.lock

# Analyze the `Cargo.lock` and `lockfile` files as cargo lockfiles
$ phylum analyze --lockfile-type cargo Cargo.lock lockfile
```
