---
title: Analyzing Dependencies
category: 6255e67693d5200013b1fa41
hidden: false
---

The Phylum CLI supports processing many different lockfiles:

| Lockfile type | Filenames                                                              |
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

The lockfile type will be automatically detected based on the filename. If needed, this can be overridden with the
`--lockfile-type` (`-t`) option.

Lockfiles can also automatically be generated for certain manifest files. See [lockfile-generation][] for details.

[lockfile-generation]: https://docs.phylum.io/docs/lockfile-generation

After setting up a Phylum [project](https://docs.phylum.io/docs/phylum_init), you can begin analysis by running:

```sh
phylum analyze
```

The default response will provide you with a high-level overview of your packages, including the total project score, score distributions across all packages, whether or not this analysis was a pass or fail and the total number of packages still processing.

```console
❯ phylum analyze
✅ Successfully parsed lockfile "./requirements.txt" as type: pip
✅ Successfully parsed lockfile "./package-lock.json" as type: npm
✅ Job ID: bbb6edae-e50b-4c6e-8386-34a5a56508e7


          Project: example-project                                         Label: uncategorized
       Proj Score: 32                                                       Date: 2023-03-23 21:22:58 UTC
         Num Deps: 58                                                     Job ID: bbb6edae-e50b-4c6e-8386-34a5a56508e7
       Ecosystems: NPM, PyPI

     Score       Count
     91 - 100  [   55] ████████████████████████████████████████████████████████                        Thresholds:
     81 - 90   [    0]                                                                              Project Score: 60
     71 - 80   [    0]                                                                    Malicious Code Risk MAL: 60
     61 - 70   [    0]                                                                     Vulnerability Risk VLN: 60
     51 - 60   [    2] ████████████████                                                      Engineering Risk ENG: 60
     41 - 50   [    0]                                                                            Author Risk AUT: 60
     31 - 40   [    1] ████████                                                                  License Risk LIC: 60
     21 - 30   [    0]
     11 - 20   [    0]
      0 - 10   [    0]

           Status: FAIL
           Reason: Project failed due to project_score threshold of 60 not being met
```

You can get more detailed output from the analysis, to include specific issues and their severity, by using the `--verbose` flag:

```sh
phylum analyze --verbose
```

If you prefer JSON formatted output, you can leverage the `--json` flag.

```sh
phylum analyze --verbose --json > output.json
```

If the analysis failed to meet the project's thresholds, the command's exit code will be set to `100`.
