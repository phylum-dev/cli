# Changelog

All notable changes are documented in this file.
The sections should follow the order `Packaging`, `Added`, `Changed`, `Fixed` and `Removed`.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

## 5.0.0 - 2023-04-13

### Added

- Parameter `ignoredPackages` to `getJobStatus`, for analysis result filtering

### Changed

- Renamed `Package.package_type` to `Package.type`
- Return types for `parseLockfile` and `getJobStatus`

### Removed

- Parameter `package_type` on `analyze`
