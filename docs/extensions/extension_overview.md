---
title: Phylum CLI Extensions
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
* [Extension Sandboxing](https://docs.phylum.io/docs/extension_sandboxing)
* [Direct Phylum API Requests](https://docs.phylum.io/docs/extension_rest_api)

> **TIP:** More info
>
> Additional how-to articles for the extension framework can be found
> [here](https://dev.to/phylum).

## Official Extensions

Official Phylum CLI extensions can be found [on GitHub]. These are a great place
to get started if you want to try out some CLI extensions or write your own.

[on GitHub]: https://github.com/phylum-dev/cli/tree/main/extensions

Additionally, many of the official extensions are distributed with the Phylum
CLI and should already be available for use. The pre-installed extensions are:

* [`npm`](https://github.com/phylum-dev/cli/tree/main/extensions/npm)
* [`pip`](https://github.com/phylum-dev/cli/tree/main/extensions/pip)
* [`poetry`](https://github.com/phylum-dev/cli/tree/main/extensions/poetry)
* [`yarn`](https://github.com/phylum-dev/cli/tree/main/extensions/yarn)
* [`bundle`](https://github.com/phylum-dev/cli/tree/main/extensions/bundle)
* [`cargo`](https://github.com/phylum-dev/cli/tree/main/extensions/cargo)
