---
title: Lockfile Generation
category: 6255e67693d5200013b1fa41
hidden: false
---

The Phylum CLI uses lockfile generators when it is given a manifest file with no matching lockfile.

### Lockfile generators

| Lockfile type | Filenames        | Required tool               |
| ------------- | ---------        | -------------               |
| `npm`         | `package.json`   | [`npm`][npm]                |
| `yarn`        | `package.json`   | [`yarn`][yarn]              |
| `pip`         | `requirements*.txt` <br/> `requirements.in` <br/> `setup.py` <br/> `setup.cfg` <br/> `pyproject.toml` | `pip-compile` (from [`pip-tools`][pip-tools]) |
| `pipenv`      | `Pipfile`        | [`pipenv`][pipenv]          |
| `poetry`      | `pyproject.toml` | [`poetry`][poetry]          |
| `gem`         | `Gemfile`        | `bundle` (from [Bundler][]) |
| `mvn`         | `pom.xml`        | `mvn` (from [Maven][])      |
| `gradle`      | `build.gradle`   | [`gradle`][gradle]          |
| `go`          | `go.mod`         | [`go`][go]                  |
| `cargo`       | `Cargo.toml`     | [`cargo`][cargo]            |

[npm]: https://nodejs.org/
[yarn]: https://yarnpkg.com/
[pip-tools]: https://github.com/jazzband/pip-tools/
[pipenv]: https://github.com/pypa/pipenv
[poetry]: https://python-poetry.org/
[bundler]: https://bundler.io/
[maven]: https://maven.apache.org/
[gradle]: https://gradle.org/
[go]: https://go.dev/
[cargo]: https://www.rust-lang.org/

For files that can be handled by multiple generators, a fallback is used:

* `package.json` will use `npm`
* `pyproject.toml` will use `poetry`

This can be overridden on the command line with the `--lockfile-type` (`-t`) option. For example:

```
phylum analyze -t yarn package.json
```

### Lockifests

Special handling is given to manifests that, for historical reasons, can also be used as lockfiles. Specifically,
Python's `requirements.txt` is a manifest file. But in some scenarios it may be fully specified and effectively becomes
a lockfile (e.g., `pip freeze > requirements.txt`).

Phylum handles these files by first attempting to analyze them as a lockfile. If anything in the file is not fully
specified, this will fail, and Phylum will silence the error and proceed to lockfile generation.

### Example scenario

1. A user runs `phylum analyze package.json`
2. The CLI checks for the existence of a matching lockfile
   (i.e., `package-lock.json`, `npm-shrinkwrap.json`, or `yarn.lock`)
3. If a matching lockfile is found, that file will be used instead
4. If no matching lockfile is found, proceed to manifest file generation
5. Since no `--lockfile-type` was specified, the fallback will be used (in this case, `npm`)
6. The lockfile generator runs this command to generate a lockfile:
   ```
   npm install --package-lock-only --ignore-scripts
   ```
7. The output lockfile (`package-lock.json`) is [analyzed][analyzing-dependencies]

[analyzing-dependencies]: https://docs.phylum.io/docs/analyzing-dependencies
