# Find extension exceptions

A [Phylum CLI][phylum-cli] extension that helps with finding required extension
sandboxing exceptions.

## Installation and basic usage

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/cli
phylum extension install cli/extensions/find-permissions
```

Run `find-permissions` against a command you want to test:

```console
phylum find-permissions --read --write --bin /usr/bin/ls
```

To find out more about the usage of `find-permissions`, check its `--help` or
visit the [extension sandboxing documentation].

[phylum-cli]: https://github.com/phylum-dev/cli
[extension sandboxing documentation]: https://cli.phylum.io/extensions/extension_sandboxing#finding-required-exceptions
