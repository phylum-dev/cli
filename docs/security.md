---
title: Security
category: 61e72e3a50a88e001a92ee5d
---

Phylum was founded by a team of security researchers at heart, and we take the security of our tooling seriously.

# Reporting security issues
We love coordinated disclosure!
Please email security@phylum.io to start a conversation! We'll coordinate a secure communication mechanism first, then evaluate the reported issue(s) and keep you apprised each step of the way.

# Disable certificate checking on `phylum` CLI
We really hope you don't have to do this, but some organizations still use SSL termination in a way where we can't reasonably enforce certificate pinning without breaking use of our CLI tool. 
You can use the `--no-check-certificate` argument to the CLI tool to disable certificate checking.
