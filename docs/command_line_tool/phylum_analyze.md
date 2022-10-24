---
title: phylum analyze
category: 6255e67693d5200013b1fa3e
hidden: false
---

Submit a request for analysis to the processing system

```sh
phylum analyze [OPTIONS] <lockfile>
```

### Options
`-F`, `--force`
&emsp; Force re-processing of packages (even if they already exist in the system)

`-f`, `--filter <filter>`
&emsp; Provide a filter used to limit the issues displayed

`-g`, `--group <group_name>`
&emsp; Specify a group to use for analysis

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-l`, `--label <label>`
&emsp; Specify a label for a given analysis submission

`-p`, `--project <project_name>`
&emsp; Specify a project to use for analysis (must already exist)

`-v`, `--verbose`
&emsp; Increase verbosity of API response

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
