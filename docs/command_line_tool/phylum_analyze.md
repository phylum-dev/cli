---
title: phylum analyze
category: 6255e67693d5200013b1fa3e
hidden: false
---

Submit a request for analysis to the processing system

```sh
Usage: phylum analyze [OPTIONS] [LOCKFILE]
```

### Arguments

[LOCKFILE]
&emsp; The package lock file to submit.

### Options

-F, --force
&emsp; Force re-processing of packages (even if they already exist in the system)

-l, --label <label>
&emsp; Specify a label to use for analysis

-f, --filter <filter>
&emsp; Provide a filter used to limit the issues displayed

&nbsp;&nbsp;&nbsp;&nbsp;EXAMPLES:
&nbsp;&nbsp;&nbsp;&nbsp;\# Show only issues with severity of at least 'high'
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=high

&nbsp;&nbsp;&nbsp;&nbsp;\# Show issues with severity of 'critical' in the 'author'
&nbsp;&nbsp;&nbsp;&nbsp;and 'engineering' domains
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;--filter=crit,aut,eng

-j, --json
&emsp; Produce output in json format (default: false)

-p, --project <project_name>
&emsp; Specify a project to use for analysis

-g, --group <group_name>
&emsp; Specify a group to use for analysis

-t, --lockfile-type <type>
&emsp; The type of the lock file (default: auto)
&emsp; Accepted values: `yarn`, `npm`, `gem`, `pip`, `pipenv`, `poetry`, `mvn`, `gradle`, `nuget`, `go`, `cargo`, `auto`

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

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
