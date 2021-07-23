# cli
Command line interface for the Phylum API

## Overview
```
❯ phylum
phylum 0.0.7
Phylum, Inc.
Client interface to the Phylum system

USAGE:
    phylum [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <FILE>        Sets a custom config file
    -t, --timeout <TIMEOUT>    Set the timeout (in seconds) for requests to the Phylum api

SUBCOMMANDS:
    analyze     Submit a request for analysis to the processing system
    auth        Manage authentication, registration, and API keys
    help        Prints this message or the help of the given subcommand(s)
    history     Return information about historical scans
    package     Retrieve the details of a specific packge
    ping        Ping the remote system to verify it is available
    projects    Create, list, link and set thresholds for projects
    update      Check for a new release of the Phylum CLI tool and update if one exists
    version     Display application version
```

## Installation
Releases of phylum are statically-built for linux x64. If you need another architecture, see the section below on Building.

An install script is provided for both releases and git versions. The script creates a `$HOME/.phylum` directory and copies the required files.

This script also **adds the .phylum directory to the user's PATH** on bash.

## Building
Phylum is written in Rust, so you'll need a recent Rust installation to build it. [Install Rust](https://www.rust-lang.org/tools/install)
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

To register a user account, use the `auth register` subcommand to enter the user registration workflow where the phylum tool will query for user input:
```sh
❯ phylum auth register

✔ Your name · demo
✔ Email address · demo@example.com
✔ Password · ********
✅ Successfully registered a new account!
```

## Activation
To have your Phylum user account enabled, please contact someone at Phylum.

## Example: First project submission
Package submissions must be part of _projects_. Projects in phylum correspond to software projects the users want to analyze. When a phylum project is created, the `.phylum_project` file is written to the current working directory and used correlate mulitple analysis jobs to a single software project.
```sh
❯ phylum projects create <project_name>
```
Next, submit a package lock file:
```sh
❯ phylum analyze package-lock.json
✅ Job ID: ec95dbc1-bd13-41f5-88f2-18ac9bcab3b6


          Project: demo-project                                            Label: uncategorized
       Proj Score: 52                                                       Date: 2021-07-23 15:30:42 UTC
         Num Deps: 63                                                     Job ID: ec95dbc1-bd13-41f5-88f2-18ac9bcab3b6
             Type: NPM                                                  Language: Javascript
          User ID: demo@example.com                            View in Phylum UI: https://app.phylum.io/ec95dbc1-bd13-41f5-88f2-18ac9bcab3b6

     Score       Count
      0 - 10   [    0]                                                                                  Project Score: 0.6
     10 - 20   [    0]                                                                        Malicious Code Risk MAL:   0
     20 - 30   [    0]                                                                         Vulnerability Risk VLN:   0
     30 - 40   [    0]                                                                           Engineering Risk ENG:   0
     40 - 50   [    0]                                                                                Author Risk AUT:   0
     50 - 60   [    2] █                                                                             License Risk LIC:   0
     60 - 70   [    0]
     70 - 80   [    1]
     80 - 90   [    0]
     90 - 100  [   60] ████████████████████████████████

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

