# cli
Command line interface for the Phylum API

## Overview
```
phylum-cli 0.0.4
Phylum, Inc.
Client interface to the Phylum system

USAGE:
    phylum-cli [OPTIONS] [SUBCOMMAND]

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
    ping          Ping the remote system to verify it is available
    register      Register a new system user
    status        Polls the system for request / job / package status
    submit        Submits a request to the processing system
    tokens        Manage API tokens
    version       Display application version

```

## Building
Phylum-cli is written in Rust, so you'll need a recent Rust installation to build it.
1. Clone repository
```sh
git clone https://github.com/phylum-dev/cli
```
2. Run install script in cli/lib
```sh
cd cli/lib
sh install.sh
```

## Configuration
Phylum-cli uses a configuration file located at $HOM#/.phylum/settings.yaml
The install.sh script copies a default configuration file, but requires user credentials or a token to communicate with the Phylum API.

To register
