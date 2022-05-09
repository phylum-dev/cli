---
title: phylum analyze
category: 6255e67693d5200013b1fa3e
hidden: false
---

Submit a request for analysis to the processing system

```sh
phylum analyze [options] <lockfile>
```

### Options
`-F`, `--force`
&emsp; Force re-processing of packages (even if they already exist in the system)

`--filter <filter>`
&emsp; Provide a filter used to limit the issues displayed

`-j`, `--json`
&emsp; Produce output in json format (default: false)

`-l <label>`
&emsp; Specify a label for a given analysis submission

`-p`, `--project <project_name>`
&emsp; Project to use for analysis (must already exist)

`-v`, `--verbose`
&emsp; Increase verbosity of API response

### Examples
```sh
# Analyze an npm lock file
$ phylum analyze package-lock.json

# Analyze a Maven lock file with a verbose json response
$ phylum analyze --json --verbose pom.xml

# Analyze a PyPI lock file and apply a label
$ phylum analyze -l test_branch requirements.txt

# Analyze a Poetry lock file and return the results to the 'sample' project
$ phylum analyze -p sample poetry.lock

# Analyze a RubyGems lock file and return a verbose response with only critical malware
$ phylum analyze --verbose --filter=crit,mal Gemfile.lock
```
