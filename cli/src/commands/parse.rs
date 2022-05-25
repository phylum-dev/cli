//! `phylum parse` command for lockfile parsing

use std::fs::read_to_string;
use std::io;
use std::path::Path;

use anyhow::{anyhow, Result};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::{CommandResult, ExitCode};
use crate::lockfiles::{
    parse_file, CSProj, GemLock, GradleLock, PackageLock, Parse, PipFile, Poetry, Pom,
    PyRequirements, YarnLock,
};

const LOCKFILE_PARSERS: &[(&str, &dyn Parse)] = &[
    ("yarn", &YarnLock),
    ("npm", &PackageLock),
    ("gem", &GemLock),
    ("pip", &PyRequirements),
    ("pipenv", &PipFile),
    ("poetry", &Poetry),
    ("mvn", &Pom),
    ("gradle", &GradleLock),
    ("nuget", &CSProj),
];

pub fn lockfile_types() -> Vec<&'static str> {
    LOCKFILE_PARSERS
        .iter()
        .map(|(name, _)| *name)
        .chain(["auto"])
        .collect()
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfile_type = matches.value_of("lockfile-type").unwrap_or("auto");
    // LOCKFILE is a required parameter, so .unwrap() should be safe.
    let lockfile = matches.value_of("LOCKFILE").unwrap();

    let pkgs = if lockfile_type == "auto" {
        let (pkgs, _) = try_get_packages(Path::new(lockfile))?;
        pkgs
    } else {
        let parser = LOCKFILE_PARSERS
            .iter()
            .filter_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
            .next()
            .unwrap();

        let data = read_to_string(lockfile)?;
        parser.parse(&data)?
    };

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok.into())
}
/// Attempt to get packages from an unknown lockfile type
pub fn try_get_packages(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    log::warn!(
        "Attempting to obtain packages from unrecognized lockfile type: {}",
        path.to_string_lossy()
    );

    let data = read_to_string(path)?;

    for (name, parser) in LOCKFILE_PARSERS.iter() {
        if let Ok(pkgs) = parser.parse(data.as_str()) {
            if !pkgs.is_empty() {
                log::debug!("File detected as type: {}", name);
                return Ok((pkgs, parser.package_type()));
            }
        }
    }

    Err(anyhow!("Failed to identify lockfile type"))
}

/// Determine the lockfile type based on its name and parse
/// accordingly to obtain the packages from it
pub fn get_packages_from_lockfile(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    let file = path
        .file_name()
        .and_then(|file| file.to_str())
        .ok_or_else(|| anyhow!("Lockfile path has no file name"))?;
    let ext = path.extension().and_then(|ext| ext.to_str());

    let pattern = match ext {
        Some("csproj") => ".csproj",
        _ => file,
    };

    let res = match pattern {
        "Gemfile.lock" => parse(GemLock, path)?,
        "package-lock.json" => parse(PackageLock, path)?,
        "yarn.lock" => parse(YarnLock, path)?,
        "requirements.txt" => parse(PyRequirements, path)?,
        "Pipfile" | "Pipfile.lock" => parse(PipFile, path)?,
        "poetry.lock" => parse(Poetry, path)?,
        "effective-pom.xml" => parse(Pom, path)?,
        "gradle.lockfile" => parse(GradleLock, path)?,
        ".csproj" => parse(CSProj, path)?,
        _ => try_get_packages(path)?,
    };

    log::debug!("Read {} packages from file `{}`", res.0.len(), file);

    Ok(res)
}

/// Get all packages for a specific lockfile type.
fn parse<P: Parse>(parser: P, path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    let pkg_type = parser.package_type();
    parse_file(parser, path).map(|pkgs| (pkgs, pkg_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_identify_lock_file_types() {
        let test_cases = [
            ("tests/fixtures/Gemfile.lock", PackageType::RubyGems),
            ("tests/fixtures/yarn-v1.lock", PackageType::Npm),
            ("tests/fixtures/yarn.lock", PackageType::Npm),
            ("tests/fixtures/package-lock.json", PackageType::Npm),
            ("tests/fixtures/sample.csproj", PackageType::Nuget),
            ("tests/fixtures/gradle.lockfile", PackageType::Maven),
            ("tests/fixtures/effective-pom.xml", PackageType::Maven),
            ("tests/fixtures/requirements.txt", PackageType::PyPi),
            ("tests/fixtures/Pipfile", PackageType::PyPi),
            ("tests/fixtures/Pipfile.lock", PackageType::PyPi),
            ("tests/fixtures/poetry.lock", PackageType::PyPi),
        ];

        for (file, expected_type) in &test_cases {
            let (_, pkg_type) = try_get_packages(Path::new(file)).unwrap();
            assert_eq!(pkg_type, *expected_type, "{}", file);
        }
    }
}
