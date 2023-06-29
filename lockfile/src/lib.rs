use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub use cargo::Cargo;
pub use csharp::{CSProj, PackagesLock};
pub use golang::GoSum;
use ignore::WalkBuilder;
pub use java::{GradleLock, Pom};
pub use javascript::{PackageLock, Pnpm, YarnLock};
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use phylum_types::types::package::PackageType;
pub use python::{PipFile, Poetry, PyRequirements};
pub use ruby::GemLock;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
pub use spdx::Spdx;
use walkdir::WalkDir;

mod cargo;
mod csharp;
mod golang;
mod java;
mod javascript;
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
    // This is historically called "nuget" but it's actually for MSBuild project files.
    // Nuget has its own file formats that are not currently supported.
    #[serde(alias = "nuget")]
    Msbuild,
    NugetLock,
    Go,
    Cargo,
    Spdx,
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
    /// specifies a lock file format by name.
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
            LockfileFormat::Go => "go",
            LockfileFormat::Cargo => "cargo",
            LockfileFormat::Spdx => "spdx",
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
            LockfileFormat::Go => &GoSum,
            LockfileFormat::Cargo => &Cargo,
            LockfileFormat::Spdx => &Spdx,
        }
    }

    /// Iterate over all supported lock file formats.
    pub fn iter() -> LockfileFormatIter {
        LockfileFormatIter(0)
    }
}

/// An iterator of all supported lock file formats.
pub struct LockfileFormatIter(u8);

impl Iterator for LockfileFormatIter {
    type Item = LockfileFormat;

    fn next(&mut self) -> Option<Self::Item> {
        // NOTE: Without explicit override, the lockfile generator will always pick the
        // first matching format for the manifest. To ensure best possible support,
        // common formats should be returned **before** less common ones (i.e. NPM
        // before Yarn).

        let item = match self.0 {
            0 => LockfileFormat::Npm,
            1 => LockfileFormat::Yarn,
            2 => LockfileFormat::Pnpm,
            3 => LockfileFormat::Gem,
            4 => LockfileFormat::Pip,
            5 => LockfileFormat::Poetry,
            6 => LockfileFormat::Pipenv,
            7 => LockfileFormat::Maven,
            8 => LockfileFormat::Gradle,
            9 => LockfileFormat::NugetLock,
            10 => LockfileFormat::Msbuild,
            11 => LockfileFormat::Go,
            12 => LockfileFormat::Cargo,
            13 => LockfileFormat::Spdx,
            _ => return None,
        };
        self.0 += 1;
        Some(item)
    }
}

pub trait Parse {
    /// Parse from a string.
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>>;

    /// Test if a file name could be a lock file supported by this parser.
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

/// Find lockfiles and manifests at or below the specified root directory.
///
/// Walks the directory tree and returns all recognized files.
///
/// Paths excluded by gitignore are automatically ignored.
pub fn find_lockable_files_at(root: impl AsRef<Path>) -> Vec<(PathBuf, LockfileFormat)> {
    let mut manifests: Vec<(PathBuf, _)> = Vec::new();
    let mut lockfiles = Vec::new();

    let walker = WalkBuilder::new(root).max_depth(Some(MAX_LOCKFILE_DEPTH)).build();

    // Find all lockfiles and manifests in the specified directory.
    for entry in walker.into_iter().flatten() {
        let path = entry.path();

        for format in LockfileFormat::iter() {
            let parser = format.parser();

            if parser.is_path_lockfile(path) {
                lockfiles.push((path.to_path_buf(), format));
                break;
            } else if parser.is_path_manifest(path) {
                // Select first matching format for manifests.
                manifests.push((path.to_path_buf(), format));
                break;
            }
        }
    }

    for i in (0..manifests.len()).rev() {
        let mut remove = false;

        let (manifest_path, _) = &manifests[i];

        // Filter out manifests with a lockfile in a directory above them.
        let mut lockfile_dirs = lockfiles.iter().filter_map(|(path, _)| path.parent());
        remove |= lockfile_dirs.any(|lockfile_dir| manifest_path.starts_with(lockfile_dir));

        // Filter out `setup.py` files with `pyproject.toml` present.
        if manifest_path.ends_with("setup.py") {
            remove |= manifests.iter().any(|(path, _)| {
                let dir = path.parent().unwrap();
                manifest_path.starts_with(dir) && path.ends_with("pyproject.toml")
            });
        }

        // Remove unwanted manifests.
        if remove {
            manifests.swap_remove(i);
        }
    }

    // Return all manifests and lockfiles.
    lockfiles.append(&mut manifests);
    lockfiles
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

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
            ("gradle.lockfile", LockfileFormat::Gradle),
            ("effective-pom.xml", LockfileFormat::Maven),
            ("requirements.txt", LockfileFormat::Pip),
            ("Pipfile.lock", LockfileFormat::Pipenv),
            ("poetry.lock", LockfileFormat::Poetry),
            ("go.sum", LockfileFormat::Go),
            ("Cargo.lock", LockfileFormat::Cargo),
            (".spdx.json", LockfileFormat::Spdx),
            (".spdx.yaml", LockfileFormat::Spdx),
        ];

        for (file, expected_type) in test_cases {
            let pkg_type = get_path_format(Path::new(file));
            assert_eq!(pkg_type, Some(*expected_type), "{}", file);
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
            ("go", LockfileFormat::Go),
            ("cargo", LockfileFormat::Cargo),
            ("spdx", LockfileFormat::Spdx),
        ] {
            let actual_format =
                name.parse().unwrap_or_else(|e| panic!("Could not parse {:?}: {}", name, e));
            assert_eq!(
                expected_format, actual_format,
                "{:?} should parse as {:?}",
                name, expected_format,
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
            ("go", LockfileFormat::Go),
            ("cargo", LockfileFormat::Cargo),
            ("spdx", LockfileFormat::Spdx),
        ] {
            let actual_name = format.to_string();
            assert_eq!(
                expected_name, &actual_name,
                "{:?} should to_string as {:?}",
                format, expected_name,
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
                "{:?}.name() should be {:?}",
                format,
                expected_name,
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
            (LockfileFormat::Pnpm, 1),
            (LockfileFormat::Gem, 1),
            (LockfileFormat::Pipenv, 1),
            (LockfileFormat::Poetry, 2),
            (LockfileFormat::Maven, 2),
            (LockfileFormat::Gradle, 1),
            (LockfileFormat::Msbuild, 2),
            (LockfileFormat::NugetLock, 1),
            (LockfileFormat::Go, 1),
            (LockfileFormat::Cargo, 3),
            (LockfileFormat::Spdx, 6),
        ] {
            let mut parsed_lockfiles = Vec::new();
            for lockfile in fs::read_dir("../tests/fixtures").unwrap().flatten() {
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
        let lockable_files = find_lockable_files_at(tempdir.path());

        // Compare results.
        let expected =
            vec![(tempdir.path().join("pyproject.toml").to_path_buf(), LockfileFormat::Pip)];
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
        let mut lockable_files = find_lockable_files_at(tempdir.path());

        // Compare results.
        lockable_files.sort_unstable();
        let expected = vec![
            (tempdir.path().join("a/pyproject.toml").to_path_buf(), LockfileFormat::Pip),
            (tempdir.path().join("b/setup.py").to_path_buf(), LockfileFormat::Pip),
            (tempdir.path().join("setup.py").to_path_buf(), LockfileFormat::Pip),
        ];
        assert_eq!(lockable_files, expected);
    }
}
