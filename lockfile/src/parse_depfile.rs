//! Parse generic dependency files.
use std::ffi::OsStr;
#[cfg(feature = "generator")]
use std::path::Path;
use std::path::PathBuf;

use anyhow::{anyhow, Context};
use phylum_types::types::package::{PackageDescriptor, PackageDescriptorAndLockfile};
use serde::{Deserialize, Serialize};

use crate::{LockfileFormat, Package, PackageVersion, Parse, ThirdPartyVersion};

/// Lockfile parsing error.
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    /// Dependency file is a manifest, but lockfile generation is disabled.
    #[error("Parsing {0:?} requires lockfile generation, but it was disabled through the CLI")]
    ManifestWithoutGeneration(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Lockfile with all its dependencies.
#[derive(Serialize, Deserialize, Debug)]
pub struct ParsedLockfile {
    /// User-facing lockfile identifier (i.e. a file path or name).
    #[serde(alias = "path")]
    pub id: String,
    pub packages: Vec<PackageDescriptor>,
    pub format: LockfileFormat,
}

impl ParsedLockfile {
    pub fn new(
        id: impl Into<String>,
        format: LockfileFormat,
        packages: Vec<PackageDescriptor>,
    ) -> Self {
        Self { id: id.into(), packages, format }
    }

    /// Convert packages to API's expected format.
    pub fn api_packages(&self) -> Vec<PackageDescriptorAndLockfile> {
        self.packages
            .iter()
            .map(|package_descriptor| PackageDescriptorAndLockfile {
                package_descriptor: package_descriptor.clone(),
                lockfile: Some(self.id.clone()),
            })
            .collect()
    }
}

/// Parse a dependency file.
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
    file_name: Option<&OsStr>,
    format: Option<LockfileFormat>,
    id: &str,
    _generation_path: Option<PathBuf>,
) -> Result<ParsedLockfile, ParseError> {
    // Crate a fake relative path.
    let pseudopath = file_name.map(PathBuf::from);

    // Try to determine the dependency file format.
    let format = format.or_else(|| pseudopath.as_ref().and_then(crate::get_path_format));

    // Attempt to parse with all known parsers as fallback.
    let format = match format {
        Some(format) => format,
        None => return Ok(try_get_packages(id, contents)?),
    };

    // Parse with the identified parser.
    let parser = format.parser();

    // Check if file could be a lockfile based on file name.
    let maybe_lockfile = pseudopath.as_ref().map_or(false, |path| parser.is_path_lockfile(path));

    // Attempt to parse the identified lockfile.
    let mut lockfile_error = None;
    if maybe_lockfile {
        // Parse lockfile content.
        let packages = parse_lockfile_content(contents, parser);

        match packages {
            Ok(packages) => return Ok(ParsedLockfile::new(id, format, packages)),
            // Store error on failure.
            Err(err) => lockfile_error = Some(err),
        }
    }

    // Check if file could be a manifest based on file name.
    let maybe_manifest = pseudopath.as_ref().map_or(false, |path| parser.is_path_manifest(path));

    // Generate lockfile if path might be a manifest and feature and option are
    // enabled.
    #[cfg(feature = "generator")]
    if let Some(generation_path) = _generation_path.filter(|_| maybe_manifest) {
        return Ok(generate_lockfile(&generation_path, id, format, parser)?);
    }

    // Return the original lockfile parsing error.
    match lockfile_error {
        // Report parsing errors only for lockfiles.
        Some(err) if !maybe_manifest => Err(err),
        _ => Err(ParseError::ManifestWithoutGeneration(id.into())),
    }
}

/// Attempt to get packages from an unknown lockfile type
fn try_get_packages(id: &str, contents: &str) -> Result<ParsedLockfile, anyhow::Error> {
    for format in LockfileFormat::iter() {
        let parser = format.parser();
        if let Some(packages) = parser.parse(contents).ok().filter(|pkgs| !pkgs.is_empty()) {
            log::info!("Identified lockfile type: {}", format);

            let packages = filter_packages(packages);

            return Ok(ParsedLockfile::new(id, format, packages));
        }
    }

    Err(anyhow!("Failed to identify type for lockfile {id:?}"))
}

/// Generate a lockfile from a manifest path.
#[cfg(feature = "generator")]
fn generate_lockfile(
    path: &Path,
    id: &str,
    format: LockfileFormat,
    parser: &dyn Parse,
) -> Result<ParsedLockfile, anyhow::Error> {
    // Find the generator for this format.
    let generator = match parser.generator() {
        Some(generator) => generator,
        None => return Err(anyhow!("unsupported manifest file {id:?}")),
    };

    eprintln!("Generating lockfile for manifest {id:?} using {format:?}…");

    // Generate a new lockfile.
    let canonical_path = path.canonicalize()?;
    let generated_lockfile = generator
            .generate_lockfile(&canonical_path)
            .context("Lockfile generation failed! For details, see: \
                https://docs.phylum.io/docs/lockfile_generation")?;

    // Parse the generated lockfile.
    let packages = parse_lockfile_content(&generated_lockfile, parser)?;

    Ok(ParsedLockfile::new(id, format, packages))
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
            let parsed = try_get_packages("id", &contents).unwrap();
            assert_eq!(parsed.format, expected_format, "{}", path);
        }
    }
}
