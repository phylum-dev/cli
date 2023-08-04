//! `phylum parse` command for lockfile parsing

use std::path::{Path, PathBuf};
use std::vec::IntoIter;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use phylum_lockfile::{LockfileFormat, Package, PackageVersion, Parse, ThirdPartyVersion};
use phylum_types::types::package::{PackageDescriptor, PackageDescriptorAndLockfile};
use walkdir::WalkDir;

use crate::commands::{CommandResult, ExitCode};
use crate::{config, print_user_warning};

pub struct ParsedLockfile {
    pub path: PathBuf,
    pub format: LockfileFormat,
    pub packages: Vec<PackageDescriptor>,
}

pub struct ParsedLockfileIterator {
    path: PathBuf,
    packages: IntoIter<PackageDescriptor>,
}

impl Iterator for ParsedLockfileIterator {
    type Item = PackageDescriptorAndLockfile;

    fn next(&mut self) -> Option<Self::Item> {
        self.packages.next().map(|package_descriptor| PackageDescriptorAndLockfile {
            package_descriptor,
            lockfile: Some(self.path.to_string_lossy().into_owned()),
        })
    }
}

impl IntoIterator for ParsedLockfile {
    type IntoIter = ParsedLockfileIterator;
    type Item = PackageDescriptorAndLockfile;

    fn into_iter(self) -> Self::IntoIter {
        ParsedLockfileIterator { path: self.path, packages: self.packages.into_iter() }
    }
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

    let mut pkgs: Vec<PackageDescriptorAndLockfile> = Vec::new();

    for lockfile in lockfiles {
        let parsed_lockfile = parse_lockfile(lockfile.path, Some(&lockfile.lockfile_type))?;
        pkgs.extend(parsed_lockfile.into_iter());
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
    let path = path.into();
    let format = find_lockfile_format(&path, lockfile_type);

    // Attempt to parse with all known parsers as fallback.
    let (format, lockfile) = match format {
        Some(format) => format,
        None => return try_get_packages(path),
    };

    // Parse with the identified parser.

    let parser = format.parser();

    // Attempt to parse the identified lockfile.
    let mut lockfile_error = None;
    if let Some(lockfile) = lockfile {
        // Parse lockfile content.
        let content = fs::read_to_string(&lockfile).map_err(Into::into);
        let packages = content.and_then(|content| parse_lockfile_content(&content, parser));

        match packages {
            Ok(packages) => return Ok(ParsedLockfile { path: lockfile, format, packages }),
            // Store error on failure.
            Err(err) => lockfile_error = Some(err),
        }
    }

    // If the path is neither a valid manifest nor lockfile, we abort.
    if !parser.is_path_manifest(&path) {
        // Return the original lockfile parsing error.
        match lockfile_error {
            Some(err) => return Err(err),
            // If it's neither manifest nor lockfile, `try_get_packages` is used instead.
            None => unreachable!("neither lockfile nor manifest"),
        }
    }

    // If the lockfile couldn't be parsed, or there is none, we generate a new one.

    // Find the generator for this lockfile format.
    let generator = match parser.generator() {
        Some(generator) => generator,
        None => return Err(anyhow!("unsupported manifest file {path:?}")),
    };

    eprintln!("Generating lockfile for manifest {path:?} using {format:?}…");

    // Generate a new lockfile.
    let generated_lockfile = generator.generate_lockfile(&path).context("Lockfile generation failed! For details, see: https://docs.phylum.io/docs/lockfile-generation")?;

    // Parse the generated lockfile.
    let packages = parse_lockfile_content(&generated_lockfile, parser)?;

    Ok(ParsedLockfile { path, format, packages })
}

/// Attempt to parse a lockfile.
fn parse_lockfile_content(
    content: &str,
    parser: &'static dyn Parse,
) -> Result<Vec<PackageDescriptor>> {
    let packages = parser.parse(content).context("Failed to parse lockfile")?;
    Ok(filter_packages(packages))
}

/// Find a lockfile's format.
fn find_lockfile_format(
    path: &Path,
    lockfile_type: Option<&str>,
) -> Option<(LockfileFormat, Option<PathBuf>)> {
    // Determine format from lockfile type.
    if let Some(lockfile_type) = lockfile_type.filter(|lockfile_type| lockfile_type != &"auto") {
        let format = lockfile_type.parse::<LockfileFormat>().unwrap();

        // Skip lockfile analysis when path is only valid manifest.
        let parser = format.parser();
        let lockfile =
            (!parser.is_path_manifest(path) || parser.is_path_lockfile(path)).then(|| path.into());

        return Some((format, lockfile));
    }

    // Determine format based on lockfile path.
    if let Some(format) = phylum_lockfile::get_path_format(path) {
        return Some((format, Some(path.into())));
    }

    // Determine format from manifest path.
    find_manifest_format(path)
}

/// Find a manifest file's format.
fn find_manifest_format(path: &Path) -> Option<(LockfileFormat, Option<PathBuf>)> {
    // Find project root directory.
    let canonicalized = fs::canonicalize(path).ok()?;
    let manifest_dir = canonicalized.parent()?;

    // Find lockfile formats matching this manifest.
    let mut formats =
        LockfileFormat::iter().filter(|format| format.parser().is_path_manifest(path)).peekable();

    // Store first format as fallback.
    let fallback_format = formats.peek().copied();

    // Look for formats which already have a lockfile generated.
    let manifest_lockfile = formats.find_map(|format| {
        let manifest_lockfile = WalkDir::new(manifest_dir)
            .into_iter()
            .flatten()
            .find(|entry| format.parser().is_path_lockfile(entry.path()))?;
        Some((format, manifest_lockfile.path().to_owned()))
    });

    // Return existing lockfile or format capable of generating it.
    match manifest_lockfile {
        Some((format, manifest_lockfile)) => {
            print_user_warning!("{path:?} is not a lockfile, using {manifest_lockfile:?} instead");
            Some((format, Some(manifest_lockfile)))
        },
        None => fallback_format.map(|format| (format, None)),
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
