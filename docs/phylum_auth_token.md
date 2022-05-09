---
title: phylum auth token
category: 6255e67693d5200013b1fa3e
hidden: false
---

Returns the current authentication token

```sh
phylum auth token [OPTIONS]
```

### Options
`-b`, `--bearer`
&emsp; Output the short-lived bearer token for the Phylum API

### Examples
```sh
# Output long-lived refresh token
$ phylum auth token

# Output short-lived bearer token
$ phylum auth token --bearer
```
