use std::fmt::Display;
use std::path::Path;
use std::str::FromStr;

pub use cargo::Cargo;
pub use csharp::CSProj;
pub use java::{GradleLock, Pom};
pub use javascript::{PackageLock, YarnLock};
use phylum_types::types::package::{PackageDescriptor, PackageType};
pub use python::{PipFile, Poetry, PyRequirements};
pub use ruby::GemLock;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};

mod cargo;
mod csharp;
mod java;
mod javascript;
mod parsers;
mod python;
mod ruby;

/// A file format that can be parsed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum LockfileFormat {
    Yarn,
    Npm,
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
    #[serde(rename = "nuget")]
    #[serde(alias = "msbuild")]
    Msbuild,
    Cargo,
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
            LockfileFormat::Gem => "gem",
            LockfileFormat::Pip => "pip",
            LockfileFormat::Pipenv => "pipenv",
            LockfileFormat::Poetry => "poetry",
            LockfileFormat::Maven => "mvn",
            LockfileFormat::Gradle => "gradle",
            LockfileFormat::Msbuild => "nuget",
            LockfileFormat::Cargo => "cargo",
        }
    }

    /// Get the corresponding parser for the specified format.
    pub fn parser(&self) -> &'static dyn Parse {
        match self {
            LockfileFormat::Yarn => &YarnLock,
            LockfileFormat::Npm => &PackageLock,
            LockfileFormat::Gem => &GemLock,
            LockfileFormat::Pip => &PyRequirements,
            LockfileFormat::Pipenv => &PipFile,
            LockfileFormat::Poetry => &Poetry,
            LockfileFormat::Maven => &Pom,
            LockfileFormat::Gradle => &GradleLock,
            LockfileFormat::Msbuild => &CSProj,
            LockfileFormat::Cargo => &Cargo,
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
        let item = match self.0 {
            0 => LockfileFormat::Yarn,
            1 => LockfileFormat::Npm,
            2 => LockfileFormat::Gem,
            3 => LockfileFormat::Pip,
            4 => LockfileFormat::Pipenv,
            5 => LockfileFormat::Poetry,
            6 => LockfileFormat::Maven,
            7 => LockfileFormat::Gradle,
            8 => LockfileFormat::Msbuild,
            9 => LockfileFormat::Cargo,
            _ => return None,
        };
        self.0 += 1;
        Some(item)
    }
}

pub type ParseResult = anyhow::Result<Vec<PackageDescriptor>>;

pub trait Parse {
    /// Parse from a string.
    fn parse(&self, data: &str) -> ParseResult;

    /// Indicate the type of packages parsed by this parser.
    fn package_type(&self) -> PackageType;

    /// Test if a file name could be a lock file supported by this parser.
    ///
    /// The file does not need to exist.
    fn is_path_lockfile(&self, path: &Path) -> bool;
}

/// Get the expected format of a potential lock file.
///
/// If the file name does not look like a lock file supported by this crate,
/// `None` is returned.
///
/// The file does not need to exist.
pub fn get_path_format<P: AsRef<Path>>(path: P) -> Option<LockfileFormat> {
    LockfileFormat::iter().find(|f| f.parser().is_path_lockfile(path.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_path_parser_identifies_lockfile_parsers() {
        let test_cases: &[(&str, LockfileFormat)] = &[
            ("Gemfile.lock", LockfileFormat::Gem),
            ("yarn.lock", LockfileFormat::Yarn),
            ("package-lock.json", LockfileFormat::Npm),
            ("sample.csproj", LockfileFormat::Msbuild),
            ("gradle.lockfile", LockfileFormat::Gradle),
            ("effective-pom.xml", LockfileFormat::Maven),
            ("requirements.txt", LockfileFormat::Pip),
            ("Pipfile", LockfileFormat::Pipenv),
            ("Pipfile.lock", LockfileFormat::Pipenv),
            ("poetry.lock", LockfileFormat::Poetry),
            ("Cargo.lock", LockfileFormat::Cargo),
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
            ("gem", LockfileFormat::Gem),
            ("pip", LockfileFormat::Pip),
            ("pipenv", LockfileFormat::Pipenv),
            ("poetry", LockfileFormat::Poetry),
            ("mvn", LockfileFormat::Maven),
            ("maven", LockfileFormat::Maven),
            ("gradle", LockfileFormat::Gradle),
            ("nuget", LockfileFormat::Msbuild),
            ("msbuild", LockfileFormat::Msbuild),
            ("cargo", LockfileFormat::Cargo),
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
            ("gem", LockfileFormat::Gem),
            ("pip", LockfileFormat::Pip),
            ("pipenv", LockfileFormat::Pipenv),
            ("poetry", LockfileFormat::Poetry),
            ("mvn", LockfileFormat::Maven),
            ("gradle", LockfileFormat::Gradle),
            ("nuget", LockfileFormat::Msbuild),
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
}
