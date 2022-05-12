//! `phylum parse` command for lockfile parsing

use std::io;
use std::path::Path;

use anyhow::{anyhow, Result};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::{CommandResult, ExitCode};
use crate::lockfiles::*;

type ParserResult = Result<(Vec<PackageDescriptor>, PackageType)>;

const LOCKFILE_PARSERS: &[(&str, &dyn Fn(&Path) -> ParserResult)] = &[
    ("yarn", &parse::<YarnLock>),
    ("npm", &parse::<PackageLock>),
    ("gem", &parse::<GemLock>),
    ("pip", &parse::<PyRequirements>),
    ("pipenv", &parse::<PipFile>),
    ("poetry", &parse::<Poetry>),
    ("mvn", &parse::<Pom>),
    ("gradle", &parse::<GradleDeps>),
    ("nuget", &parse::<CSProj>),
    ("auto", &get_packages_from_lockfile),
];

pub fn lockfile_types() -> Vec<&'static str> {
    LOCKFILE_PARSERS.iter().map(|(name, _)| *name).collect()
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfile_type = matches.value_of("lockfile-type").unwrap_or("auto");
    // LOCKFILE is a required parameter, so .unwrap() should be safe.
    let lockfile = matches.value_of("LOCKFILE").unwrap();

    let parser = LOCKFILE_PARSERS
        .iter()
        .filter_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
        .next()
        .unwrap();

    let (pkgs, _) = parser(Path::new(lockfile))?;

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok.into())
}
/// Attempt to get packages from an unknown lockfile type
pub fn try_get_packages(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    log::warn!(
        "Attempting to obtain packages from unrecognized lockfile type: {}",
        path.to_string_lossy()
    );

    // Try a package lock format and return the packages if there are some.
    macro_rules! try_format {
        ($lock:ident, $ty:literal) => {{
            let packages = parse::<$lock>(path).ok();
            if let Some(packages) = packages.filter(|(packages, _)| !packages.is_empty()) {
                log::debug!("Submitting file as type {}", $ty);
                return Ok(packages);
            }
        }};
    }

    try_format!(YarnLock, "yarn lock");
    try_format!(PackageLock, "package lock");
    try_format!(GemLock, "gem lock");
    try_format!(PyRequirements, "pip requirements.txt");
    try_format!(PipFile, "pip Pipfile or Pipfile.lock");
    try_format!(Poetry, "poetry lock");
    try_format!(Pom, "pom xml");
    try_format!(GradleDeps, "gradle dependencies");
    try_format!(CSProj, "csproj");

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
        "Gemfile.lock" => parse::<GemLock>(path)?,
        "package-lock.json" => parse::<PackageLock>(path)?,
        "yarn.lock" => parse::<YarnLock>(path)?,
        "requirements.txt" => parse::<PyRequirements>(path)?,
        "Pipfile" | "Pipfile.lock" => parse::<PipFile>(path)?,
        "poetry.lock" => parse::<Poetry>(path)?,
        "effective-pom.xml" => parse::<Pom>(path)?,
        "gradle-dependencies.txt" => parse::<GradleDeps>(path)?,
        ".csproj" => parse::<CSProj>(path)?,
        _ => try_get_packages(path)?,
    };

    log::debug!("Read {} packages from file `{}`", res.0.len(), file);

    Ok(res)
}

/// Get all packages for a specific lockfile type.
fn parse<P: Parseable>(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    Ok((P::new(path)?.parse()?, P::package_type()))
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
            ("tests/fixtures/gradle-dependencies.txt", PackageType::Maven),
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
