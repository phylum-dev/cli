# phylum analyze

Submit a request for analysis to the processing system

```sh
Usage: phylum analyze [OPTIONS] [DEPENDENCY_FILE]...
```

## Arguments

`[DEPENDENCY_FILE]`
&emsp; Path to the dependency file to submit

## Options

`-l`, `--label` `<LABEL>`
&emsp; Specify a label to use for analysis

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-p`, `--project` `<PROJECT_NAME>`
&emsp; Specify a project to use for analysis

`-g`, `--group` `<GROUP_NAME>`
&emsp; Specify a group to use for analysis

`-t`, `--type` `<TYPE>`
&emsp; Dependency file type used for all lockfiles (default: auto)
&emsp; Accepted values: `npm`, `yarn`, `pnpm`, `gem`, `pip`, `poetry`, `pipenv`, `mvn`, `gradle`, `msbuild`, `nugetlock`, `gomod`, `go`, `cargo`, `spdx`, `cyclonedx`, `auto`

`--skip-sandbox`
&emsp; Run lockfile generation without sandbox protection

`--no-generation`
&emsp; Disable generation of lockfiles from manifests

`-o`, `--org` `<ORG>`
&emsp; Phylum organization

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

## Details

The following order is used to determine which dependency file will be analyzed:

- CLI `DEPENDENCY_FILE` argument
- Dependency files in the `.phylum_project` file specified during `phylum init`
- Recursive filesystem search

If any of these locations provides a dependency file, no further search will be
done. Recursive filesystem search takes common ignore files like `.gitignore`
and `.ignore` into account.

## Examples

```sh
# Analyze your project's default dependency files
$ phylum analyze

# Analyze a Maven lockfile with a verbose json response
$ phylum analyze --json --verbose effective-pom.xml

# Analyze a PyPI dependency file and apply a label
$ phylum analyze --label test_branch requirements.txt

# Analyze a Poetry lockfile and return the results to the `sample` project
$ phylum analyze -p sample poetry.lock

# Analyze a NuGet lockfile using the `sample` project and `sGroup` group
$ phylum analyze -p sample -g sGroup packages.lock.json

# Analyze a RubyGems lockfile and return a verbose response with only critical malware
$ phylum analyze --verbose --filter=crit,mal Gemfile.lock

# Analyze the `Cargo.lock` and `lockfile` files as cargo dependency files
$ phylum analyze --type cargo Cargo.lock lockfile
```
