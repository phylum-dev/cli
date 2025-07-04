use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ignore::WalkBuilder;
#[cfg(feature = "generator")]
pub use lockfile_generator as generator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
pub use phylum_types;
use phylum_types::types::package::PackageType;
use purl::GenericPurl;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

pub use crate::cargo::Cargo;
pub use crate::csharp::{CSProj, PackagesConfig, PackagesLock};
pub use crate::cyclonedx::CycloneDX;
pub use crate::golang::{GoMod, GoSum};
pub use crate::java::{GradleLock, Pom};
pub use crate::javascript::{PackageLock, Pnpm, YarnLock};
pub use crate::parse_depfile::{parse_depfile, ParseError, ParsedLockfile};
pub use crate::python::{PipFile, Poetry, PyRequirements};
pub use crate::ruby::GemLock;
pub use crate::spdx::Spdx;

mod cargo;
mod csharp;
mod cyclonedx;
mod golang;
mod java;
mod javascript;
mod parse_depfile;
mod parsers;
mod python;
mod ruby;
mod spdx;

/// Maximum directory depth to recurse for finding lockfiles.
const MAX_LOCKFILE_DEPTH: usize = 5;

/// A file format that can be parsed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LockfileFormat {
    Yarn,
    Npm,
    Pnpm,
    Gem,
    Pip,
    Pipenv,
    Poetry,
    #[serde(rename = "mvn")]
    #[serde(alias = "maven")]
    Maven,
    Gradle,
    #[serde(alias = "nuget")]
    Msbuild,
    NugetLock,
    NugetConfig,
    GoMod,
    Go,
    Cargo,
    Spdx,
    CycloneDX,
}

impl FromStr for LockfileFormat {
    type Err = serde::de::value::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LockfileFormat::deserialize(s.into_deserializer())
    }
}

impl Display for LockfileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

impl LockfileFormat {
    /// Get the canonical Phylum name for this format.
    ///
    /// This is the string used in documentation and examples where the user
    /// specifies a lockfile format by name.
    ///
    /// This method returns the same value as `.to_string()`, but is const and
    /// returns a `&'static str`.
    pub const fn name(&self) -> &'static str {
        match self {
            LockfileFormat::Yarn => "yarn",
            LockfileFormat::Npm => "npm",
            LockfileFormat::Pnpm => "pnpm",
            LockfileFormat::Gem => "gem",
            LockfileFormat::Pip => "pip",
            LockfileFormat::Pipenv => "pipenv",
            LockfileFormat::Poetry => "poetry",
            LockfileFormat::Maven => "mvn",
            LockfileFormat::Gradle => "gradle",
            LockfileFormat::Msbuild => "msbuild",
            LockfileFormat::NugetLock => "nugetlock",
            LockfileFormat::NugetConfig => "nugetconfig",
            LockfileFormat::GoMod => "gomod",
            LockfileFormat::Go => "go",
            LockfileFormat::Cargo => "cargo",
            LockfileFormat::Spdx => "spdx",
            LockfileFormat::CycloneDX => "cyclonedx",
        }
    }

    /// Get the corresponding parser for the specified format.
    pub fn parser(&self) -> &'static dyn Parse {
        match self {
            LockfileFormat::Yarn => &YarnLock,
            LockfileFormat::Npm => &PackageLock,
            LockfileFormat::Pnpm => &Pnpm,
            LockfileFormat::Gem => &GemLock,
            LockfileFormat::Pip => &PyRequirements,
            LockfileFormat::Pipenv => &PipFile,
            LockfileFormat::Poetry => &Poetry,
            LockfileFormat::Maven => &Pom,
            LockfileFormat::Gradle => &GradleLock,
            LockfileFormat::Msbuild => &CSProj,
            LockfileFormat::NugetLock => &PackagesLock,
            LockfileFormat::NugetConfig => &PackagesConfig,
            LockfileFormat::GoMod => &GoMod,
            LockfileFormat::Go => &GoSum,
            LockfileFormat::Cargo => &Cargo,
            LockfileFormat::Spdx => &Spdx,
            LockfileFormat::CycloneDX => &CycloneDX,
        }
    }

    /// Iterate over all supported lockfile formats.
    pub fn iter() -> impl Iterator<Item = LockfileFormat> {
        // NOTE: Without explicit override, the lockfile generator will always pick the
        // first matching format for the manifest. To ensure best possible support,
        // common formats should be returned **before** less common ones (i.e. NPM
        // before Yarn).
        const FORMATS: &[LockfileFormat] = &[
            LockfileFormat::Npm,
            LockfileFormat::Yarn,
            LockfileFormat::Pnpm,
            LockfileFormat::Gem,
            LockfileFormat::Pip,
            LockfileFormat::Poetry,
            LockfileFormat::Pipenv,
            LockfileFormat::Maven,
            LockfileFormat::Gradle,
            LockfileFormat::Msbuild,
            LockfileFormat::NugetLock,
            LockfileFormat::NugetConfig,
            LockfileFormat::GoMod,
            LockfileFormat::Go,
            LockfileFormat::Cargo,
            LockfileFormat::Spdx,
            LockfileFormat::CycloneDX,
        ];

        FORMATS.iter().copied()
    }
}

pub trait Parse {
    /// Parse from a string.
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>>;

    /// Test if a file name could be a lockfile supported by this parser.
    ///
    /// The file does not need to exist.
    fn is_path_lockfile(&self, path: &Path) -> bool;

    /// Test if a file name could be a manifest file corresponding to this
    /// parser.
    ///
    /// The file does not need to exist.
    fn is_path_manifest(&self, path: &Path) -> bool;

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        None
    }
}

/// Single package parsed from a lockfile.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Package {
    pub name: String,
    pub version: PackageVersion,
    pub package_type: PackageType,
}

/// Version for a lockfile's package.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PackageVersion {
    /// Version from the ecosystem's first-party registry.
    FirstParty(String),
    /// Version from a foreign package registry.
    ThirdParty(ThirdPartyVersion),
    /// Version available through the filesystem.
    Path(Option<PathBuf>),
    /// Version distributed through GIT.
    Git(String),
    /// Version distributed over HTTP(S).
    DownloadUrl(String),
    /// Untracked version source.
    ///
    /// Used for package versions which cannot clearly be identified, like
    /// Python's local version.
    Unknown,
}

/// Version from a foreign package registry.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ThirdPartyVersion {
    pub version: String,
    pub registry: String,
}

/// Identify a lockfile's format based on its path.
///
/// Returns `None` if no supported format could be identified.
///
/// The file does not need to exist.
pub fn get_path_format<P: AsRef<Path>>(path: P) -> Option<LockfileFormat> {
    LockfileFormat::iter().find(|f| f.parser().is_path_lockfile(path.as_ref()))
}

/// Identify a dependency file's format based on its path.
///
/// Returns `None` if no supported format could be identified.
///
/// The file does not need to exist.
pub fn get_depfile_path_format<P: AsRef<Path>>(path: P) -> Option<LockfileFormat> {
    let path = path.as_ref();
    LockfileFormat::iter().find(|format| {
        let parser = format.parser();
        parser.is_path_lockfile(path) || parser.is_path_manifest(path)
    })
}

/// Find a manifest file's lockfile.
///
/// Returns `None` if no lockfile exists or the format isn't supported.
///
/// Contrary to [`get_path_format`], the `path` argument must point to an
/// existing manifest file within the project to find its lockfile.
pub fn find_manifest_lockfile<P: AsRef<Path>>(path: P) -> Option<(PathBuf, LockfileFormat)> {
    // Canonicalize the path, so calling `parent` always works.
    let path = path.as_ref();
    let canonicalized = fs::canonicalize(path).ok()?;
    let manifest_dir = canonicalized.parent()?;

    // Find matching format and lockfile in the manifest's directory.
    LockfileFormat::iter()
        // Check if file is a valid manifest.
        .filter(|format| format.parser().is_path_manifest(path))
        // Try to find the associated lockfile.
        .find_map(|format| {
            let lockfile_path = WalkDir::new(manifest_dir)
                .into_iter()
                .flatten()
                .find(|entry| format.parser().is_path_lockfile(entry.path()))?;
            Some((lockfile_path.path().to_owned(), format))
        })
}

/// Find lockfiles in the current directory subtree.
///
/// Walks the directory tree and returns all paths recognized as lockfiles.
///
/// Paths excluded by gitignore are automatically ignored.
pub fn find_lockfiles() -> Vec<(PathBuf, LockfileFormat)> {
    find_lockfiles_at(".")
}

/// Find lockfiles at or below the specified root directory.
///
/// Walks the directory tree and returns all paths recognized as lockfiles.
///
/// Paths excluded by gitignore are automatically ignored.
pub fn find_lockfiles_at(root: impl AsRef<Path>) -> Vec<(PathBuf, LockfileFormat)> {
    let walker = WalkBuilder::new(root).max_depth(Some(MAX_LOCKFILE_DEPTH)).build();
    walker
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            get_path_format(&path).map(|format| (path, format))
        })
        .collect()
}

/// Collection of lockfiles and manifests.
#[derive(Serialize)]
pub struct DepFiles {
    pub lockfiles: Vec<(PathBuf, LockfileFormat)>,
    pub manifests: Vec<(PathBuf, LockfileFormat)>,
}

impl DepFiles {
    /// Find dependency files at or below the specified root directory.
    ///
    /// Walks the directory tree and returns all recognized files.
    ///
    /// Paths excluded by gitignore are automatically ignored.
    pub fn find_at(root: impl AsRef<Path>) -> Self {
        let mut depfiles = Self { lockfiles: Vec::new(), manifests: Vec::new() };

        let walker = WalkBuilder::new(root).max_depth(Some(MAX_LOCKFILE_DEPTH)).build();

        // Find all lockfiles and manifests in the specified directory.
        for entry in walker.into_iter().flatten() {
            let path = entry.path();

            for format in LockfileFormat::iter() {
                let parser = format.parser();

                let mut format_found = false;
                // GoMod can represent a manifest and lockfile which causes duplicate
                // lockfiles being submitted when a go.sum is present. This removes
                // go.mod files from being automatically recognized as a lockfile.
                if format != LockfileFormat::GoMod && parser.is_path_lockfile(path) {
                    depfiles.lockfiles.push((path.to_path_buf(), format));
                    format_found = true;
                }

                if parser.is_path_manifest(path) {
                    // Select first matching format for manifests.
                    depfiles.manifests.push((path.to_path_buf(), format));
                    format_found = true;
                }

                // Avoid classifying lockable file as multiple formats.
                if format_found {
                    break;
                }
            }
        }

        depfiles
    }
}

/// Find dependency files at or below the specified root directory.
///
/// Walks the directory tree and returns all recognized files.
///
/// This will filter out manifests if there is a manifest or lockfile in a
/// directory above them. To get all lockfiles and manifests, see
/// [`DepFiles::find_at`].
///
/// Paths excluded by gitignore are automatically ignored.
pub fn find_depfiles_at(root: impl AsRef<Path>) -> Vec<(PathBuf, LockfileFormat)> {
    let mut depfiles = DepFiles::find_at(root);

    for i in (0..depfiles.manifests.len()).rev() {
        let mut remove = false;

        let (manifest_path, _) = &depfiles.manifests[i];

        // Filter out manifest if there's a lockfile with a matching format at or above
        // the manifest.
        let mut lockfile_dirs =
            depfiles.lockfiles.iter().filter_map(|(path, format)| Some((path.parent()?, format)));
        remove |= lockfile_dirs.any(|(mut lockfile_dir, lockfile_format)| {
            // Legacy Gradle (before v7) lockfiles are in a subdirectory,
            // so we truncate these directories to get the effective
            // directory these lockfiles were created for.
            if lockfile_format == &LockfileFormat::Gradle
                && lockfile_dir.ends_with("gradle/dependency-locks")
            {
                lockfile_dir = lockfile_dir.parent().unwrap().parent().unwrap();
            }

            lockfile_format.parser().is_path_manifest(manifest_path)
                && manifest_path.starts_with(lockfile_dir)
        });

        // Filter out manifest if there's a manifest with a matching format above the
        // manifest.
        let mut manifest_dirs = depfiles.manifests.iter().filter_map(|(path, format)| {
            let parent = path.parent()?;
            (path != manifest_path).then_some((parent, format))
        });
        if let Some(manifest_parent) = manifest_path.parent().and_then(|path| path.parent()) {
            remove |= manifest_dirs.any(|(manifest_dir, manifest_format)| {
                manifest_format.parser().is_path_manifest(manifest_path)
                    && manifest_parent.starts_with(manifest_dir)
            });
        }

        // Filter out `setup.py` files with `pyproject.toml` present.
        if manifest_path.ends_with("setup.py") {
            remove |= depfiles.manifests.iter().any(|(path, _)| {
                let dir = path.parent().unwrap();
                manifest_path.starts_with(dir) && path.ends_with("pyproject.toml")
            });
        }

        // Remove unwanted manifests.
        if remove {
            depfiles.manifests.swap_remove(i);
        }
    }

    // Return all manifests and lockfiles.
    depfiles.lockfiles.append(&mut depfiles.manifests);
    depfiles.lockfiles
}

/// Define a custom error for unknown ecosystems.
#[derive(Error, Debug)]
#[error("Could not determine ecosystem")]
pub(crate) struct UnknownEcosystem;

/// Generates a formatted package name based on the given package type and Purl.
///
/// This function formats package names differently depending on the package
/// type:
///
/// - For `Maven` packages, the format is `"namespace:name"`.
/// - For `Npm` and `Golang` packages, the format is `"namespace/name"`.
/// - For other package types, or if no namespace is provided, it defaults to
///   the package name.
///
/// # Arguments
///
/// - `package_type`: The type of the package.
/// - `purl`: A reference to the Purl struct which contains details about the
///   package.
///
/// # Returns
///
/// - A `String` representation of the formatted package name.
pub(crate) fn formatted_package_name(
    package_type: &PackageType,
    purl: &GenericPurl<String>,
) -> String {
    match (package_type, purl.namespace()) {
        (PackageType::Maven, Some(ns)) => format!("{}:{}", ns, purl.name()),
        (PackageType::Npm | PackageType::Golang, Some(ns)) => format!("{}/{}", ns, purl.name()),
        _ => purl.name().into(),
    }
}

/// Determines the package version from Purl qualifiers.
///
/// This function parses the qualifiers of a Purl object and returns the
/// corresponding `PackageVersion` based on the provided key:
///
/// - "repository_url": returns a `ThirdParty` version.
/// - "download_url": returns a `DownloadUrl` version.
/// - "vcs_url": checks if it starts with "git+" and returns a `Git` version.
/// - For other keys or in absence of any known key, it defaults to the
///   `FirstParty` version.
///
/// # Arguments
///
/// - `purl`: A reference to the Purl struct which contains package details.
/// - `pkg_version`: The default version to use if no specific qualifier is
///   found.
///
/// # Returns
///
/// - A `PackageVersion` representing the determined version.
pub(crate) fn determine_package_version(
    pkg_version: &str,
    purl: &GenericPurl<String>,
) -> PackageVersion {
    purl.qualifiers()
        .iter()
        .find_map(|(key, value)| match key.as_ref() {
            "repository_url" => Some(PackageVersion::ThirdParty(ThirdPartyVersion {
                version: pkg_version.to_string(),
                registry: value.to_string(),
            })),
            "download_url" => Some(PackageVersion::DownloadUrl(value.to_string())),
            "vcs_url" => {
                if value.starts_with("git+") {
                    Some(PackageVersion::Git(value.to_string()))
                } else {
                    None
                }
            },
            _ => None,
        })
        .unwrap_or(PackageVersion::FirstParty(pkg_version.into()))
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    #[test]
    fn get_path_parser_identifies_lockfile_parsers() {
        let test_cases: &[(&str, LockfileFormat)] = &[
            ("Gemfile.lock", LockfileFormat::Gem),
            ("yarn.lock", LockfileFormat::Yarn),
            ("package-lock.json", LockfileFormat::Npm),
            ("npm-shrinkwrap.json", LockfileFormat::Npm),
            ("pnpm-lock.yaml", LockfileFormat::Pnpm),
            ("sample.csproj", LockfileFormat::Msbuild),
            ("packages.lock.json", LockfileFormat::NugetLock),
            ("packages.project.lock.json", LockfileFormat::NugetLock),
            ("packages.config", LockfileFormat::NugetConfig),
            ("packages.project.config", LockfileFormat::NugetConfig),
            ("gradle.lockfile", LockfileFormat::Gradle),
            ("default.lockfile", LockfileFormat::Gradle),
            ("effective-pom.xml", LockfileFormat::Maven),
            ("requirements.txt", LockfileFormat::Pip),
            ("Pipfile.lock", LockfileFormat::Pipenv),
            ("poetry.lock", LockfileFormat::Poetry),
            ("go.sum", LockfileFormat::Go),
            ("Cargo.lock", LockfileFormat::Cargo),
            (".spdx.json", LockfileFormat::Spdx),
            ("file.spdx.json", LockfileFormat::Spdx),
            (".spdx.yaml", LockfileFormat::Spdx),
            ("file.spdx.yaml", LockfileFormat::Spdx),
            ("bom.json", LockfileFormat::CycloneDX),
            ("bom.xml", LockfileFormat::CycloneDX),
        ];

        for (file, expected_type) in test_cases {
            let pkg_type = get_path_format(Path::new(file));
            assert_eq!(pkg_type, Some(*expected_type), "{file}");
        }
    }

    #[test]
    fn lockfile_format_from_str_parses_correctly() {
        for (name, expected_format) in [
            ("yarn", LockfileFormat::Yarn),
            ("npm", LockfileFormat::Npm),
            ("pnpm", LockfileFormat::Pnpm),
            ("gem", LockfileFormat::Gem),
            ("pip", LockfileFormat::Pip),
            ("pipenv", LockfileFormat::Pipenv),
            ("poetry", LockfileFormat::Poetry),
            ("mvn", LockfileFormat::Maven),
            ("maven", LockfileFormat::Maven),
            ("gradle", LockfileFormat::Gradle),
            ("nuget", LockfileFormat::Msbuild),
            ("msbuild", LockfileFormat::Msbuild),
            ("nugetlock", LockfileFormat::NugetLock),
            ("nugetconfig", LockfileFormat::NugetConfig),
            ("gomod", LockfileFormat::GoMod),
            ("go", LockfileFormat::Go),
            ("cargo", LockfileFormat::Cargo),
            ("spdx", LockfileFormat::Spdx),
            ("cyclonedx", LockfileFormat::CycloneDX),
        ] {
            let actual_format =
                name.parse().unwrap_or_else(|e| panic!("Could not parse {name:?}: {e}"));
            assert_eq!(
                expected_format, actual_format,
                "{name:?} should parse as {expected_format:?}",
            );
        }
    }

    #[test]
    fn lockfile_format_display_serializes_correctly() {
        for (expected_name, format) in [
            ("yarn", LockfileFormat::Yarn),
            ("npm", LockfileFormat::Npm),
            ("pnpm", LockfileFormat::Pnpm),
            ("gem", LockfileFormat::Gem),
            ("pip", LockfileFormat::Pip),
            ("pipenv", LockfileFormat::Pipenv),
            ("poetry", LockfileFormat::Poetry),
            ("mvn", LockfileFormat::Maven),
            ("gradle", LockfileFormat::Gradle),
            ("msbuild", LockfileFormat::Msbuild),
            ("nugetlock", LockfileFormat::NugetLock),
            ("nugetconfig", LockfileFormat::NugetConfig),
            ("gomod", LockfileFormat::GoMod),
            ("go", LockfileFormat::Go),
            ("cargo", LockfileFormat::Cargo),
            ("spdx", LockfileFormat::Spdx),
            ("cyclonedx", LockfileFormat::CycloneDX),
        ] {
            let actual_name = format.to_string();
            assert_eq!(
                expected_name, &actual_name,
                "{format:?} should to_string as {expected_name:?}",
            );
        }
    }

    #[test]
    fn lockfile_format_name_matches_to_string() {
        for format in LockfileFormat::iter() {
            let expected_name = format.to_string();
            assert_eq!(
                &expected_name,
                format.name(),
                "{format:?}.name() should be {expected_name:?}",
            );
        }
    }

    /// Ensure no new lockfiles are accidentally picked up by an unrelated
    /// parser.
    #[test]
    fn parsers_only_parse_their_lockfiles() {
        for (format, lockfile_count) in [
            (LockfileFormat::Yarn, 4),
            (LockfileFormat::Npm, 2),
            (LockfileFormat::Pnpm, 3),
            (LockfileFormat::Gem, 1),
            (LockfileFormat::Pipenv, 1),
            (LockfileFormat::Poetry, 2),
            (LockfileFormat::Maven, 3),
            (LockfileFormat::Gradle, 1),
            (LockfileFormat::Msbuild, 2),
            (LockfileFormat::NugetLock, 1),
            (LockfileFormat::NugetConfig, 1),
            (LockfileFormat::GoMod, 1),
            (LockfileFormat::Go, 1),
            (LockfileFormat::Cargo, 3),
            (LockfileFormat::Spdx, 6),
            (LockfileFormat::CycloneDX, 7),
        ] {
            let mut parsed_lockfiles = Vec::new();
            for lockfile in fs::read_dir("../tests/fixtures").unwrap().flatten() {
                // Skip directories.
                if lockfile.file_type().unwrap().is_dir() {
                    continue;
                }

                let lockfile_path = lockfile.path();
                let lockfile_content = fs::read_to_string(&lockfile_path).unwrap();

                let packages = match format.parser().parse(&lockfile_content) {
                    Ok(packages) => packages,
                    Err(_) => continue,
                };

                if !packages.is_empty() {
                    parsed_lockfiles.push(lockfile_path.display().to_string());
                }
            }
            assert_eq!(
                parsed_lockfiles.len(),
                lockfile_count,
                "{format:?} successfully parsed: {parsed_lockfiles:?}"
            );
        }
    }

    #[test]
    fn skip_setup_with_pyproject() {
        // Create desired directory structure.
        let tempdir = tempfile::tempdir().unwrap();
        let files = [
            tempdir.path().join("pyproject.toml"),
            tempdir.path().join("setup.py"),
            tempdir.path().join("a/setup.py"),
        ];
        for file in &files {
            let dir = file.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            File::create(file).unwrap();
        }

        // Find manifest files.
        let lockable_files = find_depfiles_at(tempdir.path());

        // Compare results.
        let expected = vec![(tempdir.path().join("pyproject.toml"), LockfileFormat::Pip)];
        assert_eq!(lockable_files, expected);
    }

    #[test]
    fn setup_without_pyproject() {
        // Create desired directory structure.
        let tempdir = tempfile::tempdir().unwrap();
        let files = [
            tempdir.path().join("setup.py"),
            tempdir.path().join("b/setup.py"),
            tempdir.path().join("a/pyproject.toml"),
        ];
        for file in &files {
            let dir = file.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            File::create(file).unwrap();
        }

        // Find manifest files.
        let mut lockable_files = find_depfiles_at(tempdir.path());

        // Compare results.
        lockable_files.sort_unstable();
        let expected = vec![(tempdir.path().join("setup.py"), LockfileFormat::Pip)];
        assert_eq!(lockable_files, expected);
    }

    #[test]
    fn no_duplicate_requirements() {
        // Create desired directory structure.
        let tempdir = tempfile::tempdir().unwrap();
        let files = [tempdir.path().join("requirements.txt")];
        for file in &files {
            let dir = file.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            File::create(file).unwrap();
        }

        // Find lockable files.
        let lockable_files = find_depfiles_at(tempdir.path());

        // Ensure requirements.txt is only reported once.
        let expected = vec![(files[0].clone(), LockfileFormat::Pip)];
        assert_eq!(lockable_files, expected);
    }

    #[test]
    fn skip_go_mod() {
        // Create desired directory structure.
        let tempdir = tempfile::tempdir().unwrap();
        let files = [tempdir.path().join("go.mod"), tempdir.path().join("go.sum")];
        for file in &files {
            let dir = file.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            File::create(file).unwrap();
        }

        // Find lockfile.
        let lockable_files = find_depfiles_at(tempdir.path());

        // Compare results.
        let expected = vec![(tempdir.path().join("go.sum"), LockfileFormat::Go)];
        assert_eq!(lockable_files, expected);
    }

    #[test]
    fn allow_go_mod_manifest() {
        // Create desired directory structure.
        let tempdir = tempfile::tempdir().unwrap();
        let files = [tempdir.path().join("go.mod"), tempdir.path().join("main.go")];
        for file in &files {
            let dir = file.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            File::create(file).unwrap();
        }

        // Find manifest file.
        let lockable_files = find_depfiles_at(tempdir.path());

        // Compare results.
        let expected = vec![(tempdir.path().join("go.mod"), LockfileFormat::GoMod)];
        assert_eq!(lockable_files, expected);
    }

    #[test]
    fn skip_build_gradle_with_legacy_lockfiles() {
        // Create desired directory structure.
        let tempdir = tempfile::tempdir().unwrap();
        let files = [
            tempdir.path().join("build.gradle"),
            tempdir.path().join("gradle/dependency-locks/compile.lockfile"),
            tempdir.path().join("gradle/dependency-locks/default.lockfile"),
            tempdir.path().join("gradle/dependency-locks/runtime.lockfile"),
        ];
        for file in &files {
            let dir = file.parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            File::create(file).unwrap();
        }

        // Find lockfiles.
        let mut lockfiles = find_depfiles_at(tempdir.path());

        // Compare results.
        lockfiles.sort_unstable();
        let expected = vec![
            (
                tempdir.path().join("gradle/dependency-locks/compile.lockfile"),
                LockfileFormat::Gradle,
            ),
            (
                tempdir.path().join("gradle/dependency-locks/default.lockfile"),
                LockfileFormat::Gradle,
            ),
            (
                tempdir.path().join("gradle/dependency-locks/runtime.lockfile"),
                LockfileFormat::Gradle,
            ),
        ];
        assert_eq!(lockfiles, expected);
    }
}
