//! `phylum parse` command for lockfile parsing

use std::path::Path;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::{CommandResult, ExitCode};
use crate::lockfiles::*;

type ParserResult = Result<(Vec<PackageDescriptor>, PackageType)>;

const LOCKFILE_PARSERS: &[(&str, &dyn Fn(String) -> ParserResult)] = &[
    ("yarn", &parse::<YarnLock>),
    ("npm", &parse::<PackageLock>),
    ("gem", &parse::<GemLock>),
    ("pip", &parse::<PyRequirements>),
    ("pipenv", &parse::<PipFile>),
    ("poetry", &parse::<Poetry>),
    ("mvn", &parse::<Pom>),
    ("gradle", &parse::<GradleDeps>),
    ("nuget", &parse::<CSProj>),
];

pub fn lockfile_types() -> Vec<&'static str> {
    LOCKFILE_PARSERS.iter().map(|(name, _)| *name).collect()
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfile_type = matches.value_of("lockfile-type").unwrap_or("auto");
    // LOCKFILE is a required parameter, so .unwrap() should be safe.
    let lockfile = matches.value_of("LOCKFILE").unwrap();

    let (pkgs, _) = if lockfile_type == "auto" {
        get_packages_from_lockfile(Path::new(lockfile))?
    } else {
        let parser = LOCKFILE_PARSERS
            .iter()
            .filter_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
            .next()
            .unwrap();

        let lockfile_content = fs::read_to_string(lockfile)?;
        parser(lockfile_content)?
    };

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok.into())
}
/// Attempt to get packages from an unknown lockfile type
pub fn try_get_packages(lockfile_content: String) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    for (ty, parser) in LOCKFILE_PARSERS.iter().filter(|(ty, _)| ty != &"auto") {
        let packages = parser(lockfile_content.clone()).ok();
        if let Some(packages) = packages.filter(|(packages, _)| !packages.is_empty()) {
            log::debug!("Submitting file as type {}", ty);
            return Ok(packages);
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

    let lockfile_content = fs::read_to_string(path)
        .with_context(|| format!("Unable to read lockfile path {:?}", path))?;

    let res = match pattern {
        "Gemfile.lock" => parse::<GemLock>(lockfile_content)?,
        "package-lock.json" => parse::<PackageLock>(lockfile_content)?,
        "yarn.lock" => parse::<YarnLock>(lockfile_content)?,
        "requirements.txt" => parse::<PyRequirements>(lockfile_content)?,
        "Pipfile" | "Pipfile.lock" => parse::<PipFile>(lockfile_content)?,
        "poetry.lock" => parse::<Poetry>(lockfile_content)?,
        "effective-pom.xml" => parse::<Pom>(lockfile_content)?,
        "gradle-dependencies.txt" => parse::<GradleDeps>(lockfile_content)?,
        ".csproj" => parse::<CSProj>(lockfile_content)?,
        _ => {
            log::warn!(
                "Attempting to obtain packages from unrecognized lockfile type: {}",
                path.to_string_lossy()
            );

            try_get_packages(lockfile_content)?
        }
    };

    log::debug!("Read {} packages from file `{}`", res.0.len(), file);

    Ok(res)
}

/// Get all packages in a lockfile.
fn parse<P: Parseable>(text: String) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    Ok((P::from_string(text).parse()?, P::package_type()))
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
            let content = fs::read_to_string(file).unwrap();
            let (_, pkg_type) = try_get_packages(content).unwrap();
            assert_eq!(pkg_type, *expected_type, "{}", file);
        }
    }
}
