# Changelog

Notable changes to the extension API are documented in this file.

The sections should follow the order `Packaging`, `Added`, `Changed`, `Fixed` and `Removed`.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

### Changed

- `PhylumApi.parseLockfile` now adds a relative path to each package,
    allowing for Phlum's UI to display the correct lockfile path for the job

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
