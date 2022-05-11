use std::path::Path;

use anyhow::{anyhow, Result};
use phylum_types::types::package::*;

use crate::lockfiles::*;

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
        "Pipfile.txt" | "Pipfile.lock" => parse::<PipFile>(path)?,
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
pub fn parse<P: Parseable>(path: &Path) -> Result<(Vec<PackageDescriptor>, PackageType)> {
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
