# phylum extension run

Run an extension from a directory

```sh
Usage: phylum extension run [OPTIONS] <PATH> [OPTIONS]...
```

## Arguments

`<PATH>`

`[OPTIONS]`
&emsp; Extension parameters

## Options

`-y`, `--yes`
&emsp; Automatically accept requested permissions

`-o`, `--org` `<ORG>`
&emsp; Phylum organization

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

## Details

The extension will be run without prior installation.

The first set of options are for the `run` command. The second set of options
are for the extension.

## Examples

```sh
phylum extension run --yes ./my-extension --help
```
