# Changelog

Notable changes to the extension API are documented in this file.

The sections should follow the order `Packaging`, `Added`, `Changed`, `Fixed` and `Removed`.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

## 7.1.0 - 2024-09-24

## 7.0.0 - 2024-09-17

### Added

- `organization?` parameter for the following endpoints:
    - `PhylumApi::analyze`
    - `PhylumApi::getProjects`
    - `PhylumApi::createProject`
    - `PhylumApi::deleteProject`

### Changed

- Group projects are included in `PhylumApi::getProjects` with no group specified

## 6.4.0 - 2024-05-28

### Changed

- Expose API via global `Phylum` object

## 6.1.0 - 2024-01-29

### Added

- Accept PURLs in `PhylumApi::analyze`

## 6.0.0 - 2023-12-13

### Changed

- Renamed `parseLockfile` to `PhylumApi::parseDependencyFile`
- Removed `lockfile` field from `PhylumApi::Package` type
- Removed `PhylumApi::Lockfile` type in favor of `PhylumApi::DependencyFile`
- Changed `PhylumApi::analyze` packages type to `PhylumApi::PackageWithOrigin`

## 5.9.0 - 2023-12-05

### Added

- `generateLockfiles` parameter for `parseLockfile` to inhibit lockfile generation
- `sandboxGeneration` parameter for `parseLockfile` to disable the lockfile
    generation sandbox

### Fixed

- Exceptions for symlinks in `runSandboxed` on Linux
- Removing exceptions for child directories on macOS

## 5.8.0 - 2023-10-24

### Added

- Support for the upcoming repository URL feature for `PhylumApi.create_project`

## 5.7.2 - 2023-10-10

### Fixed

- Incorrect handling of `net = true` permission

## 5.7.1 - 2023-09-08

### Changed

- `PhylumApi.parseLockfile` now adds a relative path to each package,
    allowing for Phylum's UI to display the correct lockfile path for the job

## 5.7.0 - 2023-08-24

### Added

- Enable the Web Storage API (i.e., `localStorage`)

## 5.6.0 - 2023-08-08

### Added

- Added `getJobStatusRaw` and `checkPackagesRaw` APIs for detailed analysis results
- Allow `lockfile` in packages passed to `PhylumApi.analyze()`

## 5.5.0 - 2023-07-18

### Added

- New optional `label` parameter for `PhylumApi.analyze`

### Fixed

- Correctly set Content-Type header in `PhylumApi.fetch`

## 5.3.0 - 2023-06-15

### Fixed

- Uncaught extension errors now cause the CLI to exit with a non-zero exit code
- Correct the type for `status` returned by `createProject()` (actual values are "Created" or "Exists")

## 5.1.0 - 2023-05-04

### Added

- `checkPackages` function to check a list of packages against the default policy

## 5.0.0 - 2023-04-13

### Added

- Parameter `ignoredPackages` to `getJobStatus`, for analysis result filtering

### Changed

- Renamed `Package.package_type` to `Package.type`
- Return types for `parseLockfile` and `getJobStatus`

### Removed

- Parameter `package_type` on `analyze`
