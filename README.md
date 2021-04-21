# cli
Command line interface for the Phylum API

## Overview
```
phylum 0.0.6
Phylum, Inc.
Client interface to the Phylum system

USAGE:
    phylum [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <FILE>    Sets a custom config file
    -v <verbose>...        Sets the level of verbosity

SUBCOMMANDS:
    batch         Submits a batch of requests to the processing system
    cancel        Cancels a request currently in progress
    help          Prints this message or the help of the given subcommand(s)
    heuristics    List available heuristics / submit packages for heuristics
    init          Initialize a new project
    ping          Ping the remote system to verify it is available
    register      Register a new system user
    status        Polls the system for request / job / package status
    submit        Submits a request to the processing system
    tokens        Manage API tokens
    version       Display application version
```
## Releases
Currently, releases of phylum are statically-built for linux x64.

## Installation
Currently, releases of phylum are statically-built for linux x64. If you need another architecture, see the section below on Building.

An install script is provided for both releases and git versions. The script creates a `$HOME/.phylum` directory and copies the required files.

This script also **adds the .phylum directory to the user's PATH** on bash.

## Building
Phylum-cli is written in Rust, so you'll need a recent Rust installation to build it. [Install Rust](https://www.rust-lang.org/tools/install)
1. Clone repository
```sh
git clone https://github.com/phylum-dev/cli
```
2. Run build and install scripts in cli/lib
```sh
cd cli/lib
bash build.sh
bash install.sh
```

## Configuration
Phylum uses a configuration file located at `$HOME/.phylum/settings.yaml`
The install.sh script copies a default configuration file, but requires user credentials or a token to communicate with the Phylum API.

To register a user account:
```sh
phylum register -u <username (email address)> -f <first_name> -l <last_name> -p <password>
```

## Activation
To have your Phylum API user account enabled, please contact someone at Phylum.

## Example: First package submission
Package submissions must be part of _projects_. To create a project:
```sh
phylum init -p <project_name>
```
Next, submit a package like axios:
```sh
phylum submit -n axios -v 0.19.0
[Success] Job ID: 4860ff81-9b23-4e1b-8062-1cac7454f1d5
```
phylum sends the package and version information to the Phylum API and received a Job ID GUID. This GUID is then used to query for status of the specific job. In most cases, Phylum has already analyzed that package and can give an instantaneous result. In some cases, the package may need to be ingested, processed and analyzed. This might take 3-5 minutes but should happen exceedingly rarely.

Query the job status to receive an overview of the job and an overview of the package results
```sh
❯ phylum status -i 4860ff81-9b23-4e1b-8062-1cac7454f1d5
[success] Response object:
{
  "id": "4860ff81-9b23-4e1b-8062-1cac7454f1d5",
  "user_id": "3fae32db-6b59-4617-9e4d-53dc9235137f",
  "created_at": 1617384979705,
  "score": 0.5602972277915301,
  "project": "5c347fb0-0b46-4797-9c87-ed71a6d3e2cc",
  "label": "uncategorized",
  "packages": [
    {
      "name": "axios",
      "version": "0.19.0",
      "license": "MIT",
      "package_score": 0.5602972277915301,
      "num_dependencies": 30,
      "num_vulnerabilities": 2
    }
  ]
}
```
Here we can see some summary information:
* the package score of the job - `"score": 0.5602972277915301`
* the package score of the axios package specifically - `"package_score": 0.5602972277915301`
* the number of dependencies axios requires - `"num_dependencies": 30`
* the number of vulnerabilities identified in axios version 0.19.0 - `"num_vulnerabilities": 2`

In this case the package score of the job and package are the same since the job only has 1 package.

To get more detailed information about the job, we can use the verbose flag `-V` on phylum.
```sh
❯ phylum status -i 4860ff81-9b23-4e1b-8062-1cac7454f1d5 -V
[success] Response object:
{
  "id": "4860ff81-9b23-4e1b-8062-1cac7454f1d5",
  "user_id": "3fae32db-6b59-4617-9e4d-53dc9235137f",
  "created_at": 1617384979705,
  "score": 0.5602972277915301,
  "project": "5c347fb0-0b46-4797-9c87-ed71a6d3e2cc",
  "label": "uncategorized",
  "packages": [
    {
      "name": "axios",
      "version": "0.19.0",
      "license": "MIT",
      "package_score": 0.5602972277915301,
      "num_dependencies": 30,
      "num_vulnerabilities": 2,
      "type": "npm",
      "dependencies": [
<abbreviated due to length>
      ],
      "vulnerabilities": [
        {
          "base_severity": "high",
          "cve": "CVE-2020-28168",
          "description": "The `axios` NPM package before 0.21.1 contains a Server-Side Request Forgery (SSRF) vulnerability where an attacker is able to bypass a proxy by providing a URL that responds with a redirect to a restricted host or IP address.",
          "remediation": "Upgrade to 0.21.1 or later."
        },
        {
          "base_severity": "medium",
          "cve": "CVE-2020-28168",
          "description": "Axios NPM package 0.21.0 contains a Server-Side Request Forgery (SSRF) vulnerability where an attacker is able to bypass a proxy by providing a URL that responds with a redirect to a restricted host or IP address.",
          "remediation": ""
        }
      ],
      "heuristics": {
        "abandonware": {
          "description": "Finds projects that have likely been abandoned. [domain: EngineeringRisk]",
          "score": 0.9510024076542912
        },
        "license": {
          "description": "Categorizes the overall commercial risk of license used by this package. [domain: LicenseRisk]",
          "score": 0.75
        }
      }
    }
  ]
}
```
For more information, please see our [documentation](https://docs.phylum.io/docs)

