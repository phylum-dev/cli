---
title: phylum auth token
category: 6255e67693d5200013b1fa3e
parentDoc: 625758b12a1e9d0186416451
hidden: false
---

Return the current authentication token

```sh
Usage: phylum auth token [OPTIONS]
```

### Options

-b, --bearer
&emsp; Output the short-lived bearer token for the Phylum API

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Examples

```sh
# Output long-lived refresh token
$ phylum auth token

# Output short-lived bearer token
$ phylum auth token --bearer
```
