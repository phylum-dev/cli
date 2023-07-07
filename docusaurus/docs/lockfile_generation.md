# Lockfile Generation

The Phylum CLI can generate a lockfile when it is given a manifest file.

## Lockfile generators

| Lockfile type | Filenames        | Required tool               |
| ------------- | ---------        | -------------               |
| `npm`         | `package.json`   | [`npm`][npm]                |
| `yarn`        | `package.json`   | [`yarn`][yarn]              |
| `pnpm`        | `package.json`   | [`pnpm`][pnpm]              |
| `pip`         | `requirements*.txt` <br/> `requirements.in` <br/> `setup.py` <br/> `pyproject.toml` | [`pip`][pip] version 23.0.0+ |
| `pipenv`      | `Pipfile`        | [`pipenv`][pipenv]          |
| `poetry`      | `pyproject.toml` | [`poetry`][poetry]          |
| `gem`         | `Gemfile`        | `bundle` (from [Bundler][]) |
| `mvn`         | `pom.xml`        | `mvn` (from [Maven][])      |
| `gradle`      | `build.gradle`   | [`gradle`][gradle]          |
| `go`          | `go.mod`         | [`go`][go]                  |
| `cargo`       | `Cargo.toml`     | [`cargo`][cargo]            |

[npm]: https://nodejs.org
[yarn]: https://yarnpkg.com
[pnpm]: https://pnpm.io
[pip]: https://pip.pypa.io
[pipenv]: https://github.com/pypa/pipenv
[poetry]: https://python-poetry.org
[bundler]: https://bundler.io
[maven]: https://maven.apache.org
[gradle]: https://gradle.org
[go]: https://go.dev
[cargo]: https://www.rust-lang.org

:::tip

For files that can be handled by multiple generators, a fallback is used:

* `package.json` will use `npm`
* `pyproject.toml` will use `pip`

This can be overridden on the command line with the `--lockfile-type` (`-t`) option. For example:

```sh
phylum analyze -t yarn package.json
```

:::

## Lockfile detection

The Phylum CLI prefers to work directly with lockfiles if they are available. So in a few cases, the CLI will automatically switch and use the corresponding lockfile.

First, if a user runs `parse` or `analyze` on a manifest file without specifying a lockfile type, the Phylum CLI will opportunistically switch to the lockfile if it is available in the same directory. For example, `phylum analyze go.mod` will automatically switch to `go.sum` if available. To override this, simply specify a lockfile type (i.e., `phylum analyze -t go go.mod`)

Second, during automatic lockfile detection, manifest files will only be used if there is no corresponding lockfile in the same directory or any parent directory. For example, a single `Cargo.lock` file at the root of the repository will be used instead of looking at any `Cargo.toml` files anywhere in the repository. To avoid this, run `phylum init` and specify all files that you want analyzed.

## Lockifests

Special handling is given to manifests that, for historical reasons, can also be used as lockfiles. Specifically, Python's `requirements.txt` is a manifest file. But in some scenarios it may be fully specified and effectively becomes a lockfile (e.g., `pip freeze > requirements.txt`).

Phylum handles these files by first attempting to analyze them as a lockfile. If anything in the file is not fully specified, this will fail, and Phylum will silence the error and proceed to lockfile generation.

## Example scenario

1. A user runs `phylum analyze package.json`
2. The CLI checks for the existence of a matching lockfile (i.e., `package-lock.json`, `npm-shrinkwrap.json`, or `yarn.lock`)
3. If a matching lockfile is found, that file will be used instead
4. If no matching lockfile is found, proceed to manifest file generation
5. Since no `--lockfile-type` was specified, the fallback will be used (in this case, `npm`)
6. The lockfile generator runs this command to generate a lockfile:

   ```sh
   npm install --package-lock-only --ignore-scripts
   ```

7. The output lockfile (`package-lock.json`) is [analyzed](./analyzing_dependencies.md)
