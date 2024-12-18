# phylum exception remove

Remove an existing analysis exception

```sh
Usage: phylum exception remove [OPTIONS] <--group <GROUP_NAME>|--project <PROJECT_NAME>>
```

## Options

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group to remove exception from

`-p`, `--project` `<PROJECT_NAME>`
&emsp; Project to remove exceptions from

`--package-type` `<PACKAGE_TYPE>`
&emsp; Package type of the exception which should be removed
&emsp; Accepted values: `npm`, `gem`, `pypi`, `maven`, `nuget`, `golang`, `cargo`

`-n`, `--name` `<PACKAGE_NAME>`
&emsp; Fully qualified package name of the exception which should be removed

`--version` `<VERSION>`
&emsp; Package version of the exception which should be removed

`--purl` `<PURL>`
&emsp; Package in PURL format

`--id` `<ISSUE_ID>`
&emsp; Issue ID of the exception which should be removed

`--tag` `<ISSUE_TAG>`
&emsp; Issue tag of the exception which should be removed

`-o`, `--org` `<ORG>`
&emsp; Phylum organization

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help
