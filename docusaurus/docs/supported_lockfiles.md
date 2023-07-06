# Supported Lockfiles

The Phylum CLI supports processing many different lockfiles:

| Lockfile type | Filenames |
| ------------- | --------- |
| `npm`         | `package-lock.json` <br /> `npm-shrinkwrap.json` |
| `yarn`        | `yarn.lock` (Version 1 + 2) |
| `pip`         | `requirements*.txt` |
| `pipenv`      | `Pipfile.lock` |
| `poetry`      | `poetry.lock` (Version 1 + 2) |
| `gem`         | `Gemfile.lock` |
| `nuget`       | `*.csproj` |
| `mvn`         | `effective-pom.xml` |
| `gradle`      | `gradle.lockfile` |
| `go`          | `go.sum` |
| `cargo`       | `Cargo.lock` |
| `spdx`        | `*.spdx.json` <br /> `*.spdx.yaml` <br /> `*.spdx.yml` <br /> `*.spdx` |

The lockfile type will be automatically detected based on the filename. If needed, this can be overridden with the
`--lockfile-type` (`-t`) option.

Lockfiles can also automatically be generated for certain manifest files. See [lockfile_generation][] for details.

[lockfile_generation]: https://docs.phylum.io/docs/lockfile_generation
