---
title: phylum auth revoke-token
---

Revoke an API token

```sh
Usage: phylum auth revoke-token [OPTIONS] [TOKEN_NAME]...
```

### Arguments

`[TOKEN_NAME]`
&emsp; Unique token names which identify the tokens

### Options

`-v`, `--verbose`...
&emsp; Increase the level of verbosity (the maximum is -vvv)

`-q`, `--quiet`...
&emsp; Reduce the level of verbosity (the maximum is -qq)

`-h`, `--help`
&emsp; Print help

### Examples

```sh
# Interactively select tokens to revoke.
$ phylum auth revoke-token

# Revoke tokens "token1" and "token2".
$ phylum auth revoke-token "token1" "token2"
```
