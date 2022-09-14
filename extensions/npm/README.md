# Phylum npm extension

A [Phylum CLI][phylum-cli] extension that checks your [npm][npm]
dependencies through [Phylum][phylum] before installing them.

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

When invoking `phylum npm`, subcommands that would modify the `package.json` or
`package-lock.json` files will trigger a Phylum analysis.

- If the analysis is successful, the corresponding changes will be applied.
- If the analysis is unsuccessful because some of the new dependencies don't
  meet the required project thresholds, the command will fail.
- If the analysis is waiting for Phylum to process one or more of the submitted
  packages, the command will fail and the changes will _not_ be applied.
- Commands that modify neither `package.json` nor `package-lock.json` will be passed
  through to `npm` directly.

## Caveats

Unlike the `npm` CLI, this extension needs to be launched in the directory
that contains the `package.json` and `package-lock.json` files. Launching it from
any of its subdirectories will result in an error.

[phylum]: https://phylum.io
[phylum-cli]: https://github.com/phylum-dev/cli
[npm]: https://www.npmjs.com/
