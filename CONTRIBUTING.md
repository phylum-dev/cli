# Contributing to the Phylum CLI

Contributions are welcome and appreciated!

Table of Contents:

1. [Bug Reports](#bug-reports)
2. [Feature Requests](#feature-requests)
3. [Patches / Pull Requests](#patches--pull-requests)
   1. [Testing](#testing)
   2. [Documentation](#documentation)
   3. [Style](#style)
      1. [Rust](#rust)
      2. [Extensions](#extensions)
      3. [Shell scripts](#shell-scripts)
4. [Contact](#contact)

## Bug Reports

Report bugs in the [GitHub issue tracker][bugs].

[bugs]: https://github.com/phylum-dev/cli/issues/new?template=bug_report.md

Please use the template, which should remind you to include:

* A clear and concise description of the bug
* Detailed steps to reproduce the bug
* Expected behavior
* Screenshots, where appropriate
* Additional context
  * Your operating system name and version
  * Any details about your local setup that might be helpful in troubleshooting

**Security issues should be disclosed following the [security policy]**.

[security policy]: https://github.com/phylum-dev/cli/security/policy

## Feature Requests

Request new features in the [GitHub issue tracker][features], by following the
suggested template.

[features]: https://github.com/phylum-dev/cli/issues/new?template=feature_request.md

## Patches / Pull Requests

All patches have to be sent on Github as [pull requests].

All Pull Requests must have an associated issue and should include tests and
documentation where appropriate.

[pull requests]: https://github.com/phylum-dev/cli/pulls

### Testing

The CLI is tested using Rust's built in tools:

```sh
cargo test
```

To validate none of the pre-release features have been broken, you can pass the
`--all-features` flag:

```sh
cargo test --all-features
```

### Documentation

Code should be documented where appropriate. The existing code can be used as
guidance and the general `rustfmt` rules should be followed for formatting.

All user-facing CLI changes require regenerating the CLI documentation. This is
automatically validated by CI and can be done with the following command:

```sh
cargo run -p xtask gendocs
```

You can add a file to the [`doc_templates` directory](./doc_templates) to add
extra detail to a command's CLI documentation.

### Style

#### Rust

General code format is maintained using `rustfmt`:

```sh
cargo fmt
```

Some additional style lints are enforced with `clippy`:

```sh
cargo clippy
```

This should also pass for all pre-release features:

```sh
cargo clippy --all-features
```

#### Extensions

First-party extensions are written in TypeScript. Extensions code must be
formatted and checked with `deno`. See [the deno manual] for installation
instructions.

[the deno manual]: https://deno.land/manual/getting_started/installation

To apply formatting

```sh
deno fmt
```

To run lint checks

```sh
deno lint
```

#### Shell scripts

Additionally, there are a couple of script files that are part of the Phylum CLI
project. These are linted by using [shellcheck]. The following command performs
a full analysis on all scripts in the repository:

```sh
shellcheck -o all -S style -s sh $(find . -iname "*.sh")
```

[shellcheck]: https://github.com/koalaman/shellcheck

## Contact

If there are any outstanding questions about contributing to the Phylum CLI,
they can be asked on the [issue tracker].

As an alternative, you can also contact `dl-phylum-engineering@veracode.com` for
issues with using the Phylum CLI.

[issue tracker]: https://github.com/phylum-dev/cli/issues
