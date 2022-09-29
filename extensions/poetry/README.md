# Phylum Poetry extension

A [Phylum CLI][phylum-cli] extension that checks your Python [Poetry][poetry]
dependencies through [Phylum][phylum] before installing them.

## Installation

This is a pre-installed extension and may be available without any additional
action. If, for some reason, this extension is not already available, follow
these steps to install it:

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/cli
phylum extension install cli/extensions/poetry
```

## Basic usage

Prepend `phylum` to your `poetry` command invocations:

```console
phylum poetry add my-package  # This will be checked by Phylum!
```

Or set up an alias in your shell to make it transparent:

```console
alias poetry="phylum poetry"
poetry add my-package  # This will be checked by Phylum!
```

## How it works

When invoking `phylum poetry`, subcommands that would modify the
`pyproject.toml` or `poetry.lock` files will trigger a Phylum analysis.

- If the analysis is successful, the corresponding changes will be applied.
- If the analysis is unsuccessful because some of the new dependencies don't
  meet the required project thresholds, the command will fail.
- If the analysis is waiting for Phylum to process one or more of the submitted
  packages, the command will fail and the changes will _not_ be applied.
- Commands that modify neither `pyproject.toml` nor `poetry.lock` will be passed
  through to `poetry` directly.

## Caveats

Unlike the `poetry` CLI, this extension needs to be launched in the directory
that contains the `pyproject.toml` and `poetry.lock` files. Launching it from
any of its subdirectories will result in an error.

[phylum]: https://phylum.io
[phylum-cli]: https://github.com/phylum-dev/cli
[poetry]: https://python-poetry.org/
