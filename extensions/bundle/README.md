# Phylum Bundler extension

A [Phylum CLI] extension that checks your [bundle] dependencies through [Phylum]
before installing them.

## Installation

This is a pre-installed extension and may be available without any additional
action. If, for some reason, this extension is not already available, follow
these steps to install it:

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/cli
phylum extension install cli/extensions/bundle
```

## Basic usage

Prepend `phylum` to your `bundle` command invocations:

```console
phylum bundle add my-package  # This will be checked by Phylum!
```

Or set up an alias in your shell to make it transparent:

```console
alias bundle="phylum bundle"
bundle add my-package  # This will be checked by Phylum!
```

## How it works

When invoking `phylum bundle`, subcommands that would install gems will trigger
a Phylum analysis first.

- If the analysis is successful, the corresponding changes will be applied.
- If the analysis is unsuccessful because some of the new dependencies don't
  meet the required project thresholds, the command will fail.
- If the analysis is waiting for Phylum to process one or more of the submitted
  packages, the command will fail and the changes will _not_ be applied.
- Commands that modify neither `Gemfile`, nor `Gemfile.lock` will be passed
  through to `bundle` directly.

[Phylum CLI]: https://github.com/phylum-dev/cli
[Phylum]: https://phylum.io
[bundle]: https://bundler.io
