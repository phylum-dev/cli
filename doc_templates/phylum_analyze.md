{CLAP-MARKDOWN}
### Examples

```sh
# Analyze an npm lock file
$ phylum analyze package-lock.json

# Analyze a Maven lock file with a verbose json response
$ phylum analyze --json --verbose effective-pom.xml

# Analyze a PyPI lock file and apply a label
$ phylum analyze --label test_branch requirements.txt

# Analyze a Poetry lock file and return the results to the 'sample' project
$ phylum analyze -p sample poetry.lock

# Analyze a NuGet lock file using the 'sample' project and 'sGroup' group
$ phylum analyze -p sample -g sGroup app.csproj

# Analyze a RubyGems lock file and return a verbose response with only critical malware
$ phylum analyze --verbose --filter=crit,mal Gemfile.lock
```
