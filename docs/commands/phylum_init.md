# phylum init

Setup a new Phylum project

```sh
Usage: phylum init [OPTIONS] [PROJECT_NAME]
```

## Arguments

`[PROJECT_NAME]`
&emsp; Phylum project name

## Options

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group which will be the owner of the project

`-d`, `--dependency-file` `<DEPENDENCY_FILE>`
&emsp; Project-relative dependency file path

`-t`, `--type` `<TYPE>`
&emsp; Dependency file type used for all lockfiles (default: auto)
&emsp; Accepted values: `npm`, `yarn`, `pnpm`, `gem`, `pip`, `poetry`, `pipenv`, `mvn`, `gradle`, `msbuild`, `nugetlock`, `gomod`, `go`, `cargo`, `spdx`, `cyclonedx`, `auto`

`-f`, `--force`
&emsp; Overwrite existing configurations without confirmation

`-r`, `--repository-url` `<REPOSITORY_URL>`
&emsp; Repository URL of the project

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

## Examples

```sh
# Interactively initialize the Phylum project.
$ phylum init

# Create the `demo` project with a yarn lockfile and no associated group.
$ phylum init --dependency-file yarn.lock --type yarn demo
```
