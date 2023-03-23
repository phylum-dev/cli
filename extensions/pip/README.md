# Phylum pip extension

A [Phylum CLI][phylum-cli] extension that checks your [pip] dependencies
through [Phylum][phylum] before installing them.

## Installation

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/community-extensions
phylum extension install community-extensions/pip
```

## Basic usage

Prepend `phylum` to your `pip` command invocations:

```console
phylum pip install my-package  # This will be checked by Phylum!
```

Or set up an alias in your shell to make it transparent:

```console
alias pip="phylum pip"
pip install my-package  # This will be checked by Phylum!
```

## How it works

When invoking `phylum pip`, subcommands that would install new packages trigger
a Phylum analysis.

- If the analysis is successful, the corresponding changes will be applied.
- If the analysis is unsuccessful because some of the new dependencies don't
  meet the required project thresholds, the command will fail.
- If the analysis is waiting for Phylum to process one or more of the submitted
  packages, the command will fail and the changes will _not_ be applied.
- Commands that do not install any dependencies will be passed through to `pip`
  directly.

[phylum-cli]: https://github.com/phylum-dev/cli
[phylum]: https://phylum.io
[pip]: https://pip.pypa.io
