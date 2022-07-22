---
title: Extension API
category: 62c5cb137dbdad00536291a6
hidden: true
---

## Overview

Since CLI extensions are built on top of the Deno runtime, they have access to
two different APIs; the Deno API and the Phylum API.

## Deno API

Deno's API is built into the Deno runtime, providing access to all external
interfaces like Network, Disk, or the terminal console. All available
functionality is documented in [Deno's API docs]. This functionality is
available to all extensions without any imports.

Additionally, Deno also provides a complementary standard library. This includes
utility functions for several commonly used structures like collections, http,
and async. These modules can be imported using the URLs documented in Deno's
standard library documentation or by downloading them and including individual
modules as files. All standard library functionality is documented in [Deno's
standard library docs].

[Deno's API docs]: https://doc.deno.land/deno/stable
[Deno's standard library docs]: https://deno.land/std

## Phylum API

The Phylum extension API is documented in the [TypeScript module file].

[TypeScript module file]: https://github.com/phylum-dev/cli/blob/main/cli/src/extension_api.ts
