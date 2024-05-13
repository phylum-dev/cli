//! Parse generic dependency files.
use std::path::{Path, PathBuf};

#[cfg(feature = "generator")]
use anyhow::anyhow;
use anyhow::Context;
use phylum_types::types::package::PackageDescriptor;
use serde::{Deserialize, Serialize};

use crate::{LockfileFormat, Package, PackageVersion, Parse, ThirdPartyVersion};

/// Lockfile parsing error.
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    /// Dependency file is a manifest, but lockfile generation is disabled.
    #[error("Parsing {0:?} requires lockfile generation, but it was disabled")]
    ManifestWithoutGeneration(String),
    /// Dependency file is a manifest, but file type was not provided.
    #[error("Parsing {0:?} requires a type to be specified")]
    UnknownManifestFormat(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Lockfile with all its dependencies.
#[derive(Serialize, Deserialize, Debug)]
pub struct ParsedLockfile {
    pub path: String,
    pub packages: Vec<PackageDescriptor>,
    pub format: LockfileFormat,
}

impl ParsedLockfile {
    pub fn new(
        path: impl Into<String>,
        format: LockfileFormat,
        packages: Vec<PackageDescriptor>,
    ) -> Self {
        Self { path: path.into(), packages, format }
    }
}

/// Parse a dependency file.
///
/// The `path` argument is used for format identification and display purposes.
///
/// The `_generation_path` must point to the manifest on the filesystem if
/// lockfile generation should be performed. Use `None` to disable lockfile
/// generation.
///
/// # Features
///
/// Parsing manifests requires the `generator` feature.
pub fn parse_depfile(
    contents: &str,
    path: impl Into<String>,
    format: Option<LockfileFormat>,
    _generation_path: Option<PathBuf>,
) -> Result<ParsedLockfile, ParseError> {
    // Try to determine the dependency file format.
    let path = path.into();
    let format = format.or_else(|| crate::get_depfile_path_format(&path));

    // Attempt to parse with all known parsers as fallback.
    let format = match format {
        Some(format) => format,
        None => return try_get_packages(path, contents),
    };

    // Parse with the identified parser.
    let parser = format.parser();

    // Check if file could be a lockfile/manifest based on file name.
    let pseudopath = Path::new(&path);
    let maybe_lockfile = parser.is_path_lockfile(pseudopath);
    let maybe_manifest = parser.is_path_manifest(pseudopath);

    // Attempt to parse the identified lockfile.
    let mut lockfile_error = None;
    if maybe_lockfile || !maybe_manifest {
        // Parse lockfile content.
        let packages = parse_lockfile_content(contents, parser);

        match packages {
            Ok(packages) => return Ok(ParsedLockfile::new(path, format, packages)),
            // Store error on failure.
            Err(err) => lockfile_error = Some(err),
        }
    }

    // Attempt to generate a lockfile for likely manifests when feature and option
    // are enabled. This is a best effort attempt for files that are known at this
    // point to not be a valid/parseable lockfile but may parse as a manifest with
    // a non-standard name.
    #[cfg(feature = "generator")]
    if let Some(generation_path) = _generation_path.filter(|_| !maybe_lockfile || maybe_manifest) {
        if parser.generator().is_some() {
            match generate_lockfile(&generation_path, &path, format, parser) {
                Ok(depfile) => return Ok(depfile),
                // Discard errors for unknown files.
                // The error from the lockfile parser can be used instead.
                Err(_) if !maybe_manifest => {},
                Err(err) => return Err(err.into()),
            }
        }
    }

    // Return the original lockfile parsing error.
    match lockfile_error {
        // Report parsing errors only for lockfiles.
        Some(err) if !maybe_manifest => Err(err),
        _ => Err(ParseError::ManifestWithoutGeneration(path)),
    }
}

/// Attempt to get packages from an unknown lockfile type
fn try_get_packages(path: impl Into<String>, contents: &str) -> Result<ParsedLockfile, ParseError> {
    let path = path.into();
    for format in LockfileFormat::iter() {
        let parser = format.parser();
        if let Some(packages) = parser.parse(contents).ok().filter(|pkgs| !pkgs.is_empty()) {
            log::info!("Identified lockfile type: {}", format);

            let packages = filter_packages(packages);

            return Ok(ParsedLockfile::new(path, format, packages));
        }
    }

    Err(ParseError::UnknownManifestFormat(path))
}

/// Generate a lockfile from a manifest path.
#[cfg(feature = "generator")]
fn generate_lockfile(
    generation_path: &Path,
    display_path: &str,
    format: LockfileFormat,
    parser: &dyn Parse,
) -> Result<ParsedLockfile, anyhow::Error> {
    // Find the generator for this format.
    let generator = match parser.generator() {
        Some(generator) => generator,
        None => return Err(anyhow!("unsupported manifest file {display_path:?}")),
    };

    eprintln!("Generating lockfile for manifest {display_path:?} using {format:?}…");

    // Generate a new lockfile.
    let canonical_path = generation_path.canonicalize()?;
    let generated_lockfile = generator.generate_lockfile(&canonical_path).context(
        "Lockfile generation failed! For details, see: \
         https://docs.phylum.io/cli/lockfile_generation",
    )?;

    // Parse the generated lockfile.
    let packages = parse_lockfile_content(&generated_lockfile, parser)?;

    Ok(ParsedLockfile::new(display_path, format, packages))
}

/// Attempt to parse a lockfile.
fn parse_lockfile_content(
    content: &str,
    parser: &dyn Parse,
) -> Result<Vec<PackageDescriptor>, ParseError> {
    let packages = parser.parse(content).context("Failed to parse lockfile")?;
    Ok(filter_packages(packages))
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
                PackageVersion::Unknown => {
                    log::debug!("Ignoring dependency {}", package.name);
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
    use std::fs;

    use super::*;

    #[test]
    fn it_can_identify_lock_file_types() {
        let test_cases = [
            ("../tests/fixtures/Gemfile.lock", LockfileFormat::Gem),
            ("../tests/fixtures/yarn-v1.lock", LockfileFormat::Yarn),
            ("../tests/fixtures/yarn.lock", LockfileFormat::Yarn),
            ("../tests/fixtures/package-lock.json", LockfileFormat::Npm),
            ("../tests/fixtures/package-lock-v6.json", LockfileFormat::Npm),
            ("../tests/fixtures/packages.lock.json", LockfileFormat::NugetLock),
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
            ("../tests/fixtures/bom.1.3.json", LockfileFormat::CycloneDX),
            ("../tests/fixtures/bom.1.3.xml", LockfileFormat::CycloneDX),
            ("../tests/fixtures/bom.json", LockfileFormat::CycloneDX),
            ("../tests/fixtures/bom.xml", LockfileFormat::CycloneDX),
        ];

        for (path, expected_format) in test_cases {
            let contents = fs::read_to_string(path).unwrap();
            let parsed = try_get_packages(path, &contents).unwrap();
            assert_eq!(parsed.format, expected_format, "{}", path);
        }
    }
}
