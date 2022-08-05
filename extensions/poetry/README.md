# Phylum Poetry extension

A [Phylum CLI](phylum-cli) that checks your [Poetry][poetry] packages through
[Phylum][phylum] before installing them.

## Installation and basic usage

Clone the repository and install the extension via the Phylum CLI.

```console
$ git clone https://github.com/phylum-dev/cli
$ phylum extension install cli/extensions/poetry
```

Prepend `phylum` to your `poetry` command invocations, or set up an alias to
make this transparent.

```console
$ phylum poetry add my-package  # This will be checked by Phylum!
```

```console
$ alias poetry="phylum poetry"
$ poetry add my-package  # This will be checked by Phylum!
```

Commands that modify `pyproject.toml` and/or the `poetry.lock` will trigger a
Phylum analysis. If the analysis is successful, the corresponding actions will
be carried through. If the analysis is unsuccessful, the command will exit with
the error code 127. If the analysis is waiting for Phylum to process one or more
of the submitted packages, the command will exit with the error code 126.

Commands that do not modify the manifest or the lockfile will be passed through
to `poetry`.

## Caveats

Unlike the `poetry` CLI, this extension needs to be launched in the directory
that contains the `pyproject.toml` and `poetry.lock` files. Launching it from
any subdirectory will result in an error.

[phylum]: https://phylum.io
[phylum-cli]: https://github.com/phylum-dev/cli
[poetry]: https://python-poetry.org/
