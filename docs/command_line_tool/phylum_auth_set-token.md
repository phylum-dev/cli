---
title: phylum auth set-token
category: 6255e67693d5200013b1fa3e
parentDoc: 625758b12a1e9d0186416451
hidden: false
---

Set the current authentication token

```sh
Usage: phylum auth set-token [OPTIONS] [TOKEN]
```

### Arguments

[TOKEN]
&emsp; Authentication token to store (read from stdin if omitted)

### Options

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help

### Examples

```sh
# Supply the token directly on the command line
$ phylum auth set-token ph0_UyqKk8yRmuO4gRx52os3obQevBluJTGsepQw0bLRmX0

# Supply the token on stdin
$ phylum auth set-token
ph0_UyqKk8yRmuO4gRx52os3obQevBluJTGsepQw0bLRmX0
```
