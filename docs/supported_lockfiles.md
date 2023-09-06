---
title: Supported Lockfiles
category: 6255e67693d5200013b1fa41
hidden: false
---

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
| `mvn`         | `effective-pom.xml`                                                    |
| `gradle`      | `gradle.lockfile`                                                      |
| `go`          | `go.sum`                                                               |
| `cargo`       | `Cargo.lock`                                                           |
| `spdx`        | `*.spdx.json` <br /> `*.spdx.yaml` <br /> `*.spdx.yml` <br /> `*.spdx` |
| `cyclonedx`   | `*bom.json` <br /> `*bom.xml`                                          |

---

> **NOTE:**
>
> The lockfile type will be automatically detected based on the filename.
>
> If needed, this can be overridden with the `--lockfile-type` (`-t`) option.

---

> **TIP:** Manifest Support
>
> Lockfiles can also automatically be generated for certain manifest files.
> See [lockfile generation](https://docs.phylum.io/docs/lockfile_generation) for
> details.
