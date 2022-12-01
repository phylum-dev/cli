---
title: phylum auth set-token
category: 6255e67693d5200013b1fa3e
parentDoc: 625758b12a1e9d0186416451
hidden: false
---

Set the current authentication token

```sh
Usage: phylum auth set-token [OPTIONS] [token]
```

### Arguments


&emsp; Authentication token to store (read from stdin if omitted)

### Options

-v, --verbose...
&emsp; Increase the level of verbosity (the maximum is -vvv)

-q, --quiet...
&emsp; Reduce the level of verbosity (the maximum is -qq)

-h, --help
&emsp; Print help information

### Examples

```sh
# Supply the token directly on the command line
$ phylum auth set-token eyJhbGciOiJIUzI1NiJ9.eyJpYXQiOjB9.49jV8bS3WGLP20VBpCDane-kjxfGmO8L6LHgE7mLO9I

# Supply the token on stdin
$ phylum auth set-token
eyJhbGciOiJIUzI1NiJ9.eyJpYXQiOjB9.49jV8bS3WGLP20VBpCDane-kjxfGmO8L6LHgE7mLO9I
```
