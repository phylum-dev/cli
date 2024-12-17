# phylum exception add

Add a new analysis exception

```sh
Usage: phylum exception add [OPTIONS] <--group <GROUP_NAME>|--project <PROJECT_NAME>>
```

## Options

`-g`, `--group` `<GROUP_NAME>`
&emsp; Group to add exception to

`-p`, `--project` `<PROJECT_NAME>`
&emsp; Project to add exceptions to

`-e`, `--ecosystem` `<ECOSYSTEM>`
&emsp; Ecosystem of the package to add an exception for
&emsp; Accepted values: `npm`, `rubygems`, `pypi`, `maven`, `nuget`, `golang`, `cargo`

`-n`, `--name` `<PACKAGE_NAME>`
&emsp; Name and optional namespace of the package to add an exception for

`--version` `<VERSION>`
&emsp; Version of the package to add an exception for

`--purl` `<PURL>`
&emsp; Package in PURL format

`-r`, `--reason` `<REASON>`
&emsp; Reason for adding this exception

`-s`, `--no-suggestions`
&emsp; Do not query package firewall to make suggestions

`-o`, `--org` `<ORG>`
&emsp; Phylum organization

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help
