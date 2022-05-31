# Contributing to the Phylum CLI

Contributions are welcome and appreciated!

Table of Contents:

1. [Bug Reports](#bug-reports)
2. [Feature Requests](#feature-requests)
3. [Patches / Pull Requests](#patches--pull-requests)
4. [Testing](#testing)
5. [Documentation](#documentation)
6. [Style](#style)
7. [Contact](#contact)

## Bug Reports

Report bugs in the [GitHub issue tracker][bugs].

[bugs]: https://github.com/phylum-dev/cli/issues/new?template=bug_report.md

Please use the template, which should remind you to include:

* A clear and consise description of the bug
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

## Testing

The CLI is tested using Rust's built in tools:

```
cargo test
```

To validate none of the pre-release features have been broken, you can pass the
`--all-features` flag:

```
cargo test --all-features
```

## Documentation

Code should be documented where appropriate. The existing code can be used as a
guidance here and the general `rustfmt` rules can be followed for formatting.

All user-facing changes made to the CLI must also be documented in the Markdown
documentation available in the `docs/` directory.

## Style

General code format is maintained using `rustfmt`:

```
cargo fmt
```

Some additional style lints are enforced with `clippy`:

```
cargo clippy
```

This should also pass for all pre-release features:

```
cargo clippy --all-features
```

Additionally, there are a couple of script files part of the Phylum CLI project.
These are linted by using [shellcheck]. The following command performs a full
analysis on all scripts in the repository:

```
shellcheck -o all -S style -s sh $(find . -iname "*.sh")
```

[shellcheck]: https://github.com/koalaman/shellcheck

## Contact

If there are any outstanding questions about contributing to the Phylum CLI,
they can be asked on the [issue tracker].

As an alternative, you can also contact <support@phylum.io> for issues with
using the Phylum CLI.

[issue tracker]: https://github.com/alacritty/alacritty/issues
