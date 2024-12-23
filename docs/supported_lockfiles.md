# Supported Lockfiles

The Phylum CLI supports processing many different lockfiles:

| Lockfile type | Lockfiles                                                              |
| ------------- | ---------------------------------------------------------------------- |
| `npm`         | `package-lock.json` <br /> `npm-shrinkwrap.json`                       |
| `yarn`        | `yarn.lock` (Version 1 + 2)                                            |
| `pnpm`        | `pnpm-lock.yaml`                                                       |
| `pip`         | `requirements*.txt`                                                    |
| `pipenv`      | `Pipfile.lock`                                                         |
| `poetry`      | `poetry.lock` (Version 1 + 2)                                          |
| `gem`         | `Gemfile.lock`                                                         |
| `msbuild`     | `*.csproj`                                                             |
| `nugetlock`   | `packages.lock.json` <br /> `packages.*.lock.json`                     |
| `nugetconfig` | `packages.config` <br /> `packages.*.config`                           |
| `mvn`         | `effective-pom.xml`                                                    |
| `gradle`      | `gradle.lockfile` <br /> `gradle/dependency-locks/*.lockfile`          |
| `go`          | `go.sum`                                                               |
| `gomod`       | `go.mod`                                                               |
| `cargo`       | `Cargo.lock`                                                           |
| `spdx`        | `*.spdx.json` <br /> `*.spdx.yaml` <br /> `*.spdx.yml` <br /> `*.spdx` |
| `cyclonedx`   | `*bom.json` <br /> `*bom.xml`                                          |

---

> **NOTE:**
>
> The lockfile type will be automatically detected based on the filename.
>
> If needed, this can be overridden with the `--type` (`-t`) option.

---

> **TIP:** Manifest Support
>
> Lockfiles can also automatically be generated for certain manifest files.
> See [lockfile generation](./lockfile_generation.md) for details.
