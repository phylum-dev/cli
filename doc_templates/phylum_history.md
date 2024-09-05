{PH-HEADER}

{PH-MARKDOWN}

## Examples

```sh
# List the last 30 analysis runs
$ phylum history

# View the analysis results of a historical job
$ phylum history 338ea79f-0e82-4422-9769-4e583a84599f

# View a list of analysis runs for the `sample` project
$ phylum history --project sample

# Show analysis runs for the `sample` project of the `demo` group under the `test` org
$ phylum history --org test --group demo --project sample
```
