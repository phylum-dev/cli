{PH-HEADER}

{PH-MARKDOWN}

## Examples

```sh
# Show logs for packages which failed analysis for the group `demo`.
$ phylum firewall log demo --action AnalysisFailure

# Show logs which were created after 2024 for the group `demo`.
$ phylum firewall log demo --after 2024-01-01T00:00:0.0Z

# Show logs for libc regardless of its version for the group `demo`.
$ phylum firewall log demo --package pkg:cargo/libc
```
