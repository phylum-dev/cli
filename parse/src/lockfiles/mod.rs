use std::fmt::Display;
use std::path::Path;
use std::str::FromStr;

pub use csharp::CSProj;
pub use java::{GradleLock, Pom};
pub use javascript::{PackageLock, YarnLock};
use phylum_types::types::package::{PackageDescriptor, PackageType};
pub use python::{PipFile, Poetry, PyRequirements};
pub use ruby::GemLock;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

mod csharp;
mod java;
mod javascript;
mod parsers;
mod python;
mod ruby;

/// A file format that can be parsed.
#[derive(
    Clone, Copy, Debug, Deserialize, EnumIter, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum LockFileFormat {
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
}

impl FromStr for LockFileFormat {
    type Err = serde::de::value::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LockFileFormat::deserialize(s.into_deserializer())
    }
}

impl Display for LockFileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

impl LockFileFormat {
    /// Get the corresponding parser for the specified format.
    pub fn parser(&self) -> &'static dyn Parse {
        match self {
            LockFileFormat::Yarn => &YarnLock,
            LockFileFormat::Npm => &PackageLock,
            LockFileFormat::Gem => &GemLock,
            LockFileFormat::Pip => &PyRequirements,
            LockFileFormat::Pipenv => &PipFile,
            LockFileFormat::Poetry => &Poetry,
            LockFileFormat::Maven => &Pom,
            LockFileFormat::Gradle => &GradleLock,
            LockFileFormat::Msbuild => &CSProj,
        }
    }
}

pub type ParseResult = anyhow::Result<Vec<PackageDescriptor>>;

pub trait Parse {
    /// Parse from a string
    fn parse(&self, data: &str) -> ParseResult;

    /// Indicate the type of file parsed by this parser
    fn format(&self) -> LockFileFormat;

    /// Indicate the type of packages parsed by this parser
    fn package_type(&self) -> PackageType;

    /// Test if a file name could be a lock file supported by this parser.
    ///
    /// The file does not need to exist.
    fn is_path_lockfile(&self, path: &Path) -> bool;
}

/// Get the parser of a potential lock file.
///
/// If the file name does not look like a lock file supported by this crate,
/// `None` is returned.
///
/// The file does not need to exist.
pub fn get_path_parser<P: AsRef<Path>>(path: P) -> Option<&'static dyn Parse> {
    LockFileFormat::iter().map(|f| f.parser()).find(|p| p.is_path_lockfile(path.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_path_parser_identifies_lock_file_parsers() {
        let test_cases: &[(&str, LockFileFormat)] = &[
            ("Gemfile.lock", LockFileFormat::Gem),
            ("yarn.lock", LockFileFormat::Yarn),
            ("package-lock.json", LockFileFormat::Npm),
            ("sample.csproj", LockFileFormat::Msbuild),
            ("gradle.lockfile", LockFileFormat::Gradle),
            ("effective-pom.xml", LockFileFormat::Maven),
            ("requirements.txt", LockFileFormat::Pip),
            ("Pipfile", LockFileFormat::Pipenv),
            ("Pipfile.lock", LockFileFormat::Pipenv),
            ("poetry.lock", LockFileFormat::Poetry),
        ];

        for (file, expected_type) in test_cases {
            let pkg_type = get_path_parser(Path::new(file)).map(|p| p.format());
            assert_eq!(pkg_type, Some(*expected_type), "{}", file);
        }
    }

    #[test]
    fn lock_file_format_parser_gets_correct_parser() {
        // Make sure that the parser returned by LockFileFormat::parser() reports the
        // same format from its Parse::format().
        for format in LockFileFormat::iter() {
            let parser = format.parser();
            assert_eq!(format, parser.format());
        }
    }

    #[test]
    fn lock_file_format_from_str_parses_correctly() {
        for (name, expected_format) in [
            ("yarn", LockFileFormat::Yarn),
            ("npm", LockFileFormat::Npm),
            ("gem", LockFileFormat::Gem),
            ("pip", LockFileFormat::Pip),
            ("pipenv", LockFileFormat::Pipenv),
            ("poetry", LockFileFormat::Poetry),
            ("mvn", LockFileFormat::Maven),
            ("maven", LockFileFormat::Maven),
            ("gradle", LockFileFormat::Gradle),
            ("nuget", LockFileFormat::Msbuild),
            ("msbuild", LockFileFormat::Msbuild),
        ] {
            let actual_format = name.parse().expect(&format!("Could not parse {:?}", name));
            assert_eq!(
                expected_format, actual_format,
                "{:?} should parse as {:?}",
                name, expected_format,
            );
        }
    }

    #[test]
    fn lock_file_format_display_serializes_correctly() {
        for (expected_name, format) in [
            ("yarn", LockFileFormat::Yarn),
            ("npm", LockFileFormat::Npm),
            ("gem", LockFileFormat::Gem),
            ("pip", LockFileFormat::Pip),
            ("pipenv", LockFileFormat::Pipenv),
            ("poetry", LockFileFormat::Poetry),
            ("mvn", LockFileFormat::Maven),
            ("gradle", LockFileFormat::Gradle),
            ("nuget", LockFileFormat::Msbuild),
        ] {
            let actual_name = format.to_string();
            assert_eq!(
                expected_name, &actual_name,
                "{:?} should to_string as {:?}",
                format, expected_name,
            );
        }
    }
}
