# Phylum npm extension

A [Phylum CLI][phylum-cli] extension that checks your [npm][npm] dependencies
through [Phylum][phylum] before installing them.

## Installation

This is a pre-installed extension and may be available without any additional
action. If, for some reason, this extension is not already available, follow
these steps to install it:

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/cli
phylum extension install cli/extensions/npm
```

## Basic usage

Prepend `phylum` to your `npm` command invocations:

```console
phylum npm install my-package  # This will be checked by Phylum!
```

Or set up an alias in your shell to make it transparent:

```console
alias npm="phylum npm"
npm install my-package  # This will be checked by Phylum!
```

## How it works

Wher running the package manager through this extension, subcommands which would
install new packages will trigger a Phylum analysis first. Once that analysis
passes Phylum's default policy, the installation is performed. If it did not
pass the analysis, the command will return early with an error.

In cases where Phylum still needs to process some of the packages, the command
will exit with a warning **without** installing the packages. Once the analysis
is complete, another attempt can be made.

[phylum]: https://phylum.io
[phylum-cli]: https://github.com/phylum-dev/cli
[npm]: https://www.npmjs.com/
