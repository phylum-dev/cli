//! `phylum parse` command for lockfile parsing

use std::path::PathBuf;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use phylum_lockfile::{
    get_path_format, LockfileFormat, Package, PackageVersion, ThirdPartyVersion,
};
use phylum_types::types::package::PackageDescriptor;
use walkdir::WalkDir;

use crate::commands::{CommandResult, ExitCode};
use crate::{config, print_user_warning};

pub struct ParsedLockfile {
    pub path: PathBuf,
    pub format: LockfileFormat,
    pub packages: Vec<PackageDescriptor>,
}

pub fn lockfile_types(add_auto: bool) -> Vec<&'static str> {
    let mut lockfile_types = LockfileFormat::iter().map(|format| format.name()).collect::<Vec<_>>();

    // Add generic lockfile type.
    if add_auto {
        lockfile_types.push("auto");
    }

    lockfile_types
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfiles = config::lockfiles(matches, phylum_project::get_current_project().as_ref())?;

    let mut pkgs: Vec<PackageDescriptor> = Vec::new();

    for lockfile in lockfiles {
        pkgs.extend(parse_lockfile(lockfile.path, Some(&lockfile.lockfile_type))?.packages);
    }

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok)
}

/// Parse a package lockfile.
pub fn parse_lockfile(
    path: impl Into<PathBuf>,
    lockfile_type: Option<&str>,
) -> Result<ParsedLockfile> {
    // Try and determine lockfile format.
    let mut path = path.into();
    let format = lockfile_type
        .filter(|lockfile_type| lockfile_type != &"auto")
        .map(|lockfile_type| lockfile_type.parse::<LockfileFormat>().unwrap())
        .or_else(|| get_path_format(&path))
        .or_else(|| {
            let canonicalized = fs::canonicalize(&path).ok()?;
            let manifest_dir = canonicalized.parent()?;

            // Find matching format and lockfile in manifest's directory.
            let (format, lockfile_path) = LockfileFormat::iter()
                .filter(|format| format.parser().is_path_manifest(&path))
                .find_map(|format| {
                    let lockfile_path = WalkDir::new(manifest_dir)
                        .into_iter()
                        .flatten()
                        .find(|entry| format.parser().is_path_lockfile(entry.path()))?;
                    Some((format, lockfile_path))
                })?;

            // Switch manifest to lockfile path.
            let lockfile_path = lockfile_path.path().to_owned();
            print_user_warning!("{path:?} is not a lockfile, using {lockfile_path:?} instead");
            path = lockfile_path;

            Some(format)
        });

    match format {
        // Parse with identified parser.
        Some(format) => {
            let data = fs::read_to_string(&path)?;
            let parser = format.parser();

            let packages = parser
                .parse(&data)
                .with_context(|| format!("Failed to parse lockfile {path:?}"))?;
            let packages = filter_packages(packages);

            Ok(ParsedLockfile { path, packages, format })
        },
        // Attempt to parse with all parsers until success.
        None => try_get_packages(path),
    }
}

/// Attempt to get packages from an unknown lockfile type
fn try_get_packages(path: PathBuf) -> Result<ParsedLockfile> {
    let data = fs::read_to_string(&path)?;

    for format in LockfileFormat::iter() {
        let parser = format.parser();
        if let Some(packages) = parser.parse(data.as_str()).ok().filter(|pkgs| !pkgs.is_empty()) {
            log::info!("Identified lockfile type: {}", format);

            let packages = filter_packages(packages);

            return Ok(ParsedLockfile { path, packages, format });
        }
    }

    Err(anyhow!("Failed to identify type for lockfile {path:?}"))
}

/// Filter packages for submission.
fn filter_packages(mut packages: Vec<Package>) -> Vec<PackageDescriptor> {
    packages
        .drain(..)
        .filter_map(|package| {
            // Check if package should be submitted based on version format.
            let version = match package.version {
                PackageVersion::FirstParty(version) => version,
                PackageVersion::ThirdParty(ThirdPartyVersion { registry, version }) => {
                    log::debug!("Using registry {registry:?} for {} ({version})", package.name);
                    version
                },
                PackageVersion::Git(url) => {
                    log::debug!("Git dependency {} will not be analyzed ({url:?})", package.name);
                    url
                },
                PackageVersion::Path(path) => {
                    log::debug!("Ignoring filesystem dependency {} ({path:?})", package.name);
                    return None;
                },
                PackageVersion::DownloadUrl(url) => {
                    log::debug!("Ignoring remote dependency {} ({url:?})", package.name);
                    return None;
                },
            };

            Some(PackageDescriptor {
                package_type: package.package_type,
                version,
                name: package.name,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_identify_lock_file_types() {
        let test_cases = [
            ("../tests/fixtures/Gemfile.lock", LockfileFormat::Gem),
            ("../tests/fixtures/yarn-v1.lock", LockfileFormat::Yarn),
            ("../tests/fixtures/yarn.lock", LockfileFormat::Yarn),
            ("../tests/fixtures/package-lock.json", LockfileFormat::Npm),
            ("../tests/fixtures/package-lock-v6.json", LockfileFormat::Npm),
            ("../tests/fixtures/Calculator.csproj", LockfileFormat::Msbuild),
            ("../tests/fixtures/sample.csproj", LockfileFormat::Msbuild),
            ("../tests/fixtures/Calculator.csproj", LockfileFormat::Msbuild),
            ("../tests/fixtures/gradle.lockfile", LockfileFormat::Gradle),
            ("../tests/fixtures/effective-pom.xml", LockfileFormat::Maven),
            ("../tests/fixtures/workspace-effective-pom.xml", LockfileFormat::Maven),
            ("../tests/fixtures/requirements-locked.txt", LockfileFormat::Pip),
            ("../tests/fixtures/Pipfile.lock", LockfileFormat::Pipenv),
            ("../tests/fixtures/poetry.lock", LockfileFormat::Poetry),
            ("../tests/fixtures/poetry_v2.lock", LockfileFormat::Poetry),
            ("../tests/fixtures/go.sum", LockfileFormat::Go),
            ("../tests/fixtures/Cargo_v1.lock", LockfileFormat::Cargo),
            ("../tests/fixtures/Cargo_v2.lock", LockfileFormat::Cargo),
            ("../tests/fixtures/Cargo_v3.lock", LockfileFormat::Cargo),
            ("../tests/fixtures/spdx-2.2.spdx", LockfileFormat::Spdx),
            ("../tests/fixtures/spdx-2.2.spdx.json", LockfileFormat::Spdx),
            ("../tests/fixtures/spdx-2.3.spdx.json", LockfileFormat::Spdx),
            ("../tests/fixtures/spdx-2.3.spdx.yaml", LockfileFormat::Spdx),
        ];

        for (file, expected_format) in test_cases {
            let parsed = try_get_packages(PathBuf::from(file)).unwrap();
            assert_eq!(parsed.format, expected_format, "{}", file);
        }
    }
}
