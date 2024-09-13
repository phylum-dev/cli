# Phylum Poetry extension

A [Phylum CLI][phylum-cli] extension that checks your Python [Poetry][poetry]
dependencies through [Phylum][phylum] before installing them.

[phylum-cli]: https://github.com/phylum-dev/cli
[poetry]: https://python-poetry.org/
[phylum]: https://phylum.io

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

When running the package manager through this extension, subcommands which would
install new packages will trigger a Phylum analysis first. Once that analysis
passes Phylum's default policy, the installation is performed. If it did not
pass the analysis, the command will return early with an error.

In cases where Phylum still needs to process some of the packages, the command
will exit with a warning **without** installing the packages. Once the analysis
is complete, another attempt can be made.

## Troubleshooting

There are some common reasons this extension may fail.

### Sandbox violations

The underlying package manager may fail due to operating within the Phylum
sandbox. These errors may be displayed in the output and say something about not
being able to access a particular resource.

One possibility is that this is a valid sandbox violation that has not been
accounted for with an exception. Another possibility is that a malicious action
was taken by one of the packages. Blocking such malicious actions is a primary
goal for using this extension and failures of this sort are a warning to not
proceed without the protection of a sandbox.

It can be hard to know what is malicious and what is not. If you aren't sure
what to do, please [contact us][contact] and provide the log output along with
any other relevant details. If you believe an exception is needed for the
sandbox, please [submit an issue][issue].

[issue]: https://github.com/phylum-dev/cli/issues/new/choose
[contact]: https://docs.phylum.io/support/contact_us

### Package violations

Packages analyzed by this extension may not pass the
[Phylum default policy][policy]. When this happens the extension will prevent
the requested action and instead print a list of policy violations.

There is not currently a method for allowing policy violations discovered by
this extension. Instead, inspect the findings and determine if the offending
entries are truly necessary. If not, update your project dependencies and try
again. If they are, then the only recourse is to try the action again without
the Phylum package manager extension.

[policy]: https://docs.phylum.io/knowledge_base/policy
