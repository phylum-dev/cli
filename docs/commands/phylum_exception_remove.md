# phylum exception remove

Remove an existing analysis exception

```sh
Usage: phylum exception remove [OPTIONS] <--group <GROUP_NAME>|--project <PROJECT_NAME>>
```

## Options

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group to add exception to

`-p`, `--project` `<PROJECT_NAME>`
&emsp; Project to add exceptions to

`-e`, `--ecosystem` `<ECOSYSTEM>`
&emsp; Ecosystem of the exception which should be removed
&emsp; Accepted values: `npm`, `rubygems`, `pypi`, `maven`, `nuget`, `golang`, `cargo`

`-n`, `--name` `<PACKAGE_NAME>`
&emsp; Package name and optional namespace of the exception which should be removed

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
