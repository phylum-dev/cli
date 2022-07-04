---
title: Phylum CLI Extensions
category: TODO
hidden: true
---

Phylum CLI extensions are optional plugins for the CLI which provide additional
functionality in a modular fashion.

Extensions are executed in a Deno JavaScript runtime and have access to Phylum's
API for commonly used operations. The capability-based permission system,
together with Deno's sandbox, ensures that extensions can only do what they're
supposed to.

## Usage

If you're interested in using existing Phylum CLI extensions, you can take a
look at the [CLI's extension documentation].

[CLI's extension documentation]: https://docs.phylum.io/docs/phylum_extension

## Writing Extensions

* [Quickstart](https://docs.phylum.io/docs/extensions_quickstart)
* [Manifest Format](https://docs.phylum.io/docs/extensions_manifest)
* [Extension API](https://docs.phylum.io/docs/extensions_api)
* [Example](https://docs.phylum.io/docs/extensions_example)
