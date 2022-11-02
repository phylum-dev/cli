//! `phylum parse` command for lockfile parsing

use std::fs::read_to_string;
use std::io;
use std::path::Path;

use anyhow::{anyhow, Result};
use phylum_lockfile::{get_path_format, LockfileFormat};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use crate::commands::{CommandResult, ExitCode};

pub struct ParsedLockfile {
    pub format: LockfileFormat,
    pub packages: Vec<PackageDescriptor>,
    pub package_type: PackageType,
}

pub fn lockfile_types() -> Vec<&'static str> {
    LockfileFormat::iter().map(|format| format.name()).chain(["auto"]).collect()
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfile_type = matches.get_one::<String>("lockfile-type");
    // LOCKFILE is a required parameter, so .unwrap() is safe.
    let lockfile = matches.get_one::<String>("LOCKFILE").unwrap();

    let pkgs = parse_lockfile(lockfile, lockfile_type)?.packages;

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok.into())
}

/// Parse a package lockfile.
pub fn parse_lockfile(
    path: impl AsRef<Path>,
    lockfile_type: Option<&String>,
) -> Result<ParsedLockfile> {
    // Try and determine lockfile format.
    let format = lockfile_type
        .filter(|path| path.eq_ignore_ascii_case("auto"))
        .map(|lockfile_type| lockfile_type.parse::<LockfileFormat>().unwrap())
        .or_else(|| get_path_format(path.as_ref()));

    match format {
        // Parse with identified parser.
        Some(format) => {
            let data = read_to_string(path)?;
            let parser = format.parser();

            Ok(ParsedLockfile {
                format,
                packages: parser.parse(&data)?,
                package_type: parser.package_type(),
            })
        },
        // Attempt to parse with all parsers until success.
        None => try_get_packages(path.as_ref()),
    }
}

/// Attempt to get packages from an unknown lockfile type
fn try_get_packages(path: &Path) -> Result<ParsedLockfile> {
    let data = read_to_string(path)?;

    for format in LockfileFormat::iter() {
        let parser = format.parser();
        if let Ok(packages) = parser.parse(data.as_str()) {
            if !packages.is_empty() {
                log::info!("Identified lockfile type: {}", format);
                return Ok(ParsedLockfile {
                    format,
                    packages,
                    package_type: parser.package_type(),
                });
            }
        }
    }

    Err(anyhow!("Failed to identify lockfile type"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_identify_lock_file_types() {
        let test_cases = [
            ("../tests/fixtures/Gemfile.lock", PackageType::RubyGems, LockfileFormat::Gem),
            ("../tests/fixtures/yarn-v1.lock", PackageType::Npm, LockfileFormat::Yarn),
            ("../tests/fixtures/yarn.lock", PackageType::Npm, LockfileFormat::Yarn),
            ("../tests/fixtures/package-lock.json", PackageType::Npm, LockfileFormat::Npm),
            ("../tests/fixtures/sample.csproj", PackageType::Nuget, LockfileFormat::Msbuild),
            ("../tests/fixtures/gradle.lockfile", PackageType::Maven, LockfileFormat::Gradle),
            ("../tests/fixtures/effective-pom.xml", PackageType::Maven, LockfileFormat::Maven),
            (
                "../tests/fixtures/workspace-effective-pom.xml",
                PackageType::Maven,
                LockfileFormat::Maven,
            ),
            ("../tests/fixtures/requirements.txt", PackageType::PyPi, LockfileFormat::Pip),
            ("../tests/fixtures/Pipfile", PackageType::PyPi, LockfileFormat::Pipenv),
            ("../tests/fixtures/Pipfile.lock", PackageType::PyPi, LockfileFormat::Pipenv),
            ("../tests/fixtures/poetry.lock", PackageType::PyPi, LockfileFormat::Poetry),
            ("../tests/fixtures/go.sum", PackageType::Golang, LockfileFormat::Go),
            ("../tests/fixtures/Cargo_v3.lock", PackageType::Cargo, LockfileFormat::Cargo),
        ];

        for (file, expected_type, expected_format) in test_cases {
            let parsed = try_get_packages(Path::new(file)).unwrap();
            assert_eq!(parsed.package_type, expected_type, "{}", file);
            assert_eq!(parsed.format, expected_format, "{}", file);
        }
    }
}
