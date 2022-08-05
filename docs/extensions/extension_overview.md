---
title: Phylum CLI Extensions
category: 62c5cb137dbdad00536291a6
---

## Overview

Phylum CLI extensions are optional plugins for the CLI which provide additional
functionality in a modular fashion.

Extensions are executed in a [Deno] JavaScript runtime and have access to
Phylum's API for commonly used operations. The capability-based permission
system, together with Deno's sandbox, ensures that extensions can only do what
they're supposed to.

[Deno]: https://deno.land/

## Usage

If you're interested in using existing Phylum CLI extensions, you can take a
look at the [CLI's extension documentation].

[CLI's extension documentation]: https://docs.phylum.io/docs/phylum_extension

## Writing Extensions

* [Quickstart](https://docs.phylum.io/docs/extension_quickstart)
* [Manifest Format](https://docs.phylum.io/docs/extension_manifest)
* [Extension API](https://docs.phylum.io/docs/extension_api)
* [Example](https://docs.phylum.io/docs/extension_example)
