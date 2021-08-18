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
Releases of phylum are statically-built for Linux and MacOS x64. If you need another architecture, see the section below on Building.

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
phylum sends the information from the package lockfile to the Phylum API and receives summary analysis results.

Here we can see some summary information:
* the project score of the analysis job - `Proj. Score: 52`
* the number of dependencies - `Num Deps: 63`
* a histogram on the lower left showing the distribution of package scores for each dependency in the analysis job

To get more detailed information about the job, we can use the verbose flag `-V` on phylum.
```sh
❯ phylum analyze package-lock.json -V
... long output omitted ...
```

To get the analysis results data in JSON format, we can use the `--json` option:
```sh
❯ phylum analyze package-lock.json -V --json
... long output omitted ...
```

For more information, please see our [documentation](https://docs.phylum.io/docs)

