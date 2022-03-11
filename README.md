<!-- markdownlint-disable-file MD033 - inline HTML is allowed here -->
<!-- markdownlint-disable-next-line MD041 - okay to NOT start file with top-level heading -->
<p align="center">
  <img height="100" src="https://phylum.io/logo/dark-bckg.svg">
</p>

---

# Command Line Utility for [Phylum.io](https://phylum.io)
  
![GitHub release (latest by date)](https://img.shields.io/github/v/release/phylum-dev/cli)
[![MIT License](https://img.shields.io/apm/l/atomic-design-ui.svg?)](https://github.com/tterb/atomic-design-ui/blob/master/LICENSEs)
![Test Status](https://github.com/phylum-dev/cli/actions/workflows/test.yml/badge.svg?branch=master)
[![README](https://img.shields.io/badge/docs-README-yellowgreen)](https://docs.phylum.io/docs/welcome)

The command line interface (CLI) allows users to submit their project package dependencies to Phylum's API for analysis. Currently [pre-built binaries](https://github.com/phylum-dev/cli/releases) for Linux and macOS are available. For other platforms (e.g. Windows), binaries can easily be [built](https://github.com/phylum-dev/cli#building). Additional information about using the CLI tool is available in the [documentation](https://docs.phylum.io/docs/welcome).

[![asciicast](https://asciinema.org/a/431262.svg)](https://asciinema.org/a/431262)

## Quickstart for Linux or macOS

1. Download and unzip the latest release package for your target:

   | Target | Package |
   | --- | --- |
   | x86_64-unknown-linux-musl | [phylum-linux-x86_64.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-linux-x86_64.zip) |
   | x86_64-apple-darwin | [phylum-macos-x86_64.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-macos-x86_64.zip) |
   | aarch64-apple-darwin | [phylum-macos-aarch64.zip](https://github.com/phylum-dev/cli/releases/latest/download/phylum-macos-aarch64.zip) |

2. Confirm the signature of the `phylum` binary located within the archive with [minisign](https://jedisct1.github.io/minisign/) and the public key for Phylum

   ```sh
   $ minisign -Vm phylum -P RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf
   Signature and comment signature verified
   Trusted comment: Phylum - the future of software supply chain security
   ```

3. Run the installer script

   ```sh
   ./install.sh
   ```

4. [Register](https://docs.phylum.io/docs/authentication) for an account (if you don't already have one)

   ```sh
   phylum auth register
   ```

5. [Authenticate](https://docs.phylum.io/docs/authentication) with Phylum (requires [activation](https://github.com/phylum-dev/cli/tree/development#activation) by Phylum)

   ```sh
   phylum auth login
   ```

6. [Create a new Phylum project](https://docs.phylum.io/docs/projects#creating-a-new-project) in your project directory

   ```sh
   phylum projects create <project-name>
   ```

7. [Submit your package lock file](https://docs.phylum.io/docs/analyzing-dependencies)

   ```sh
   phylum analyze <package-lock-file.ext>
   ```

---

## Overview

```sh
❯ phylum
phylum 1.0.1
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

Releases of phylum are built for Linux and macOS x64. If you need another architecture, see the section below on Building.

Running the install script provided with the release package will install phylum, i.e.

```sh
./install.sh
```

This will create a new directory in `$HOME/.phylum` to house the configuration, and will place the binary in the appropriate location. The script will also add the `$HOME/.phylum` directory to the user's `PATH`.

## Building

<details>
  <summary>Expand to learn more about building from source</summary>
  
Phylum is written in Rust, so you'll need a recent Rust installation to build it (we tested with v1.58.0). [Install Rust](https://www.rust-lang.org/tools/install)

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

</details>

## Registration

<details>
  <summary>Expand to learn more about Phylum account registration</summary>

To register a user account, use the `auth register` subcommand to enter the user registration workflow where the Phylum tool will open a web browser to complete the process:

```sh
❯ phylum auth register

Please use browser window to complete login process
If browser window does not open, you can use the link below:
 <URL>
✅ Successfully registered a new account!
```

</details>

## Activation

To have your Phylum user account enabled, please contact someone at Phylum.
  
## Configuration

<details>
  <summary>Expand to learn more about the Phylum configuration file</summary>

Phylum uses a configuration file located at `$HOME/.phylum/settings.yaml`  
The `install.sh` script copies a default configuration file, but requires a token to communicate with the Phylum API. The `settings.yaml` file is automatically updated with the proper token value after a successful CLI login.  
The `offline_access` parameter in the `settings.yaml` file contains the API token. The following command can be used to retrieve your token value:  

```sh
grep "offline_access" $HOME/.phylum/settings.yaml | sed 's/  offline_access: //'
```

</details>

## Example: First project submission

Package submissions must be part of _projects_. Projects in Phylum correspond to software projects the users want to analyze. When a phylum project is created, the `.phylum_project` file is written to the current working directory and used correlate mulitple analysis jobs to a single software project.

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

## Questions/Issues

Please contact Phylum with any questions or issues using the CLI tool.

Email: <support@phylum.io>
