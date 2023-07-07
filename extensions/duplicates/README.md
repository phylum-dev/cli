# Phylum duplicates extension

A [Phylum CLI][phylum-cli] extension that checks for duplicates in your
dependency graph.

[phylum-cli]: https://github.com/phylum-dev/cli

This extension serves as a simple example and has its source code fully
explained in our [extension example documentation].

[extension example documentation]: https://cli.phylum.io/extensions/extension_example

## Installation and basic usage

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/cli
phylum extension install cli/extensions/duplicates
```

Check for duplicates by pointing the extension at your lockfile:

```console
phylum duplicates ./package-lock.json
```
