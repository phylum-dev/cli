//! `phylum parse` command for lockfile parsing

use std::ffi::OsStr;
use std::fs::read_to_string;
use std::io;
use std::path::Path;

use anyhow::{anyhow, Result};
use clap::builder::TypedValueParser;
use clap::error::{Error as ClapError, ErrorKind as ClapErrorKind};
use clap::{Arg, Command};
use phylum_lockfile::{get_path_format, LockfileFormat};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use crate::commands::{CommandResult, ExitCode};
use crate::{print_user_success, print_user_warning};

pub fn lockfile_types() -> Vec<&'static str> {
    LockfileFormat::iter().map(|format| format.name()).chain(["auto"]).collect()
}

#[derive(Copy, Clone)]
pub struct LockfileValueParser;

impl TypedValueParser for LockfileValueParser {
    type Value = &'static str;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, ClapError> {
        // Assure value is valid UTF8.
        let value = match value.to_str() {
            Some(value) => value,
            None => return Err(ClapError::raw(ClapErrorKind::InvalidUtf8, "expected UTF8 string")),
        };

        // Check if value matches one of the known lockfile types.
        for lockfile_type in LockfileFormat::iter().map(|format| format.name()).chain(["auto"]) {
            if value == lockfile_type {
                return Ok(lockfile_type);
            }
        }

        Err(ClapError::raw(
            ClapErrorKind::InvalidValue,
            format!("invalid lockfile type: {:?}", value),
        ))
    }
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfile_type = matches.get_one::<&str>("lockfile-type").unwrap_or(&"auto");
    // LOCKFILE is a required parameter, so .unwrap() should be safe.
    let lockfile = matches.get_one::<String>("LOCKFILE").unwrap();

    let pkgs = if lockfile_type == &"auto" {
        let (pkgs, _) = try_get_packages(Path::new(lockfile))?;
        pkgs
    } else {
        let parser = lockfile_type.parse::<LockfileFormat>().unwrap().parser();

        let data = read_to_string(lockfile)?;
        parser.parse(&data)?
    };

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok.into())
}
/// Attempt to get packages from an unknown lockfile type
pub fn try_get_packages(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    print_user_warning!(
        "Attempting to obtain packages from unrecognized lockfile type: {}",
        path.to_string_lossy()
    );

    let data = read_to_string(path)?;

    for format in LockfileFormat::iter() {
        let parser = format.parser();
        if let Ok(pkgs) = parser.parse(data.as_str()) {
            if !pkgs.is_empty() {
                print_user_success!("Identified lockfile type: {}", format);
                return Ok((pkgs, parser.package_type()));
            }
        }
    }

    Err(anyhow!("Failed to identify lockfile type"))
}

/// Determine the lockfile type based on its name and parse
/// accordingly to obtain the packages from it
pub fn get_packages_from_lockfile(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    let res = match get_path_format(path) {
        Some(format) => {
            let data = read_to_string(path)?;
            let parser = format.parser();
            (parser.parse(&data)?, parser.package_type())
        },
        None => try_get_packages(path)?,
    };

    log::debug!("Read {} packages from file `{}`", res.0.len(), path.display());

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_identify_lock_file_types() {
        let test_cases = [
            ("../tests/fixtures/Gemfile.lock", PackageType::RubyGems),
            ("../tests/fixtures/yarn-v1.lock", PackageType::Npm),
            ("../tests/fixtures/yarn.lock", PackageType::Npm),
            ("../tests/fixtures/package-lock.json", PackageType::Npm),
            ("../tests/fixtures/sample.csproj", PackageType::Nuget),
            ("../tests/fixtures/gradle.lockfile", PackageType::Maven),
            ("../tests/fixtures/effective-pom.xml", PackageType::Maven),
            ("../tests/fixtures/workspace-effective-pom.xml", PackageType::Maven),
            ("../tests/fixtures/requirements.txt", PackageType::PyPi),
            ("../tests/fixtures/Pipfile", PackageType::PyPi),
            ("../tests/fixtures/Pipfile.lock", PackageType::PyPi),
            ("../tests/fixtures/poetry.lock", PackageType::PyPi),
        ];

        for (file, expected_type) in &test_cases {
            let (_, pkg_type) = try_get_packages(Path::new(file)).unwrap();
            assert_eq!(pkg_type, *expected_type, "{}", file);
        }
    }
}
