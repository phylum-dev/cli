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

    let packages = YarnLock::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type yarn lock");
        return Ok((packages, PackageType::Npm));
    }

    let packages = PackageLock::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type package lock");
        return Ok((packages, PackageType::Npm));
    }

    let packages = GemLock::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type gem lock");
        return Ok((packages, PackageType::RubyGems));
    }

    let packages = PyRequirements::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type pip requirements.txt");
        return Ok((packages, PackageType::PyPi));
    }

    let packages = PipFile::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type pip Pipfile or Pipfile.lock");
        return Ok((packages, PackageType::PyPi));
    }

    let packages = Poetry::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type poetry lock");
        return Ok((packages, PackageType::PyPi));
    }

    let packages = Pom::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type pom xml");
        return Ok((packages, PackageType::Maven));
    }

    let packages = GradleDeps::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type gradle dependencies");
        return Ok((packages, PackageType::Maven));
    }

    let packages = CSProj::new(path)?.parse();
    if let Some(packages) = packages.ok().filter(|pkgs| !pkgs.is_empty()) {
        log::debug!("Submitting file as type csproj");
        return Ok((packages, PackageType::Nuget));
    }

    Err(anyhow!("Failed to identify lockfile type"))
}

/// Determine the lockfile type based on its name and parse
/// accordingly to obtain the packages from it
pub fn get_packages_from_lockfile(path: &str) -> Result<(Vec<PackageDescriptor>, PackageType)> {
    let path = Path::new(path);
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
        "Gemfile.lock" => {
            let parser = GemLock::new(path)?;
            (parser.parse()?, PackageType::RubyGems)
        }
        "package-lock.json" => {
            let parser = PackageLock::new(path)?;
            (parser.parse()?, PackageType::Npm)
        }
        "yarn.lock" => {
            let parser = YarnLock::new(path)?;
            (parser.parse()?, PackageType::Npm)
        }
        "requirements.txt" => {
            let parser = PyRequirements::new(path)?;
            (parser.parse()?, PackageType::PyPi)
        }
        "Pipfile.txt" | "Pipfile.lock" => {
            let parser = PipFile::new(path)?;
            (parser.parse()?, PackageType::PyPi)
        }
        "poetry.lock" => {
            let parser = Poetry::new(path)?;
            (parser.parse()?, PackageType::PyPi)
        }
        "effective-pom.xml" => {
            let parser = Pom::new(path)?;
            (parser.parse()?, PackageType::Maven)
        }
        "gradle-dependencies.txt" => {
            let parser = GradleDeps::new(path)?;
            (parser.parse()?, PackageType::Maven)
        }
        ".csproj" => {
            let parser = CSProj::new(path)?;
            (parser.parse()?, PackageType::Nuget)
        }
        _ => try_get_packages(path)?,
    };

    log::debug!("Read {} packages from file `{}`", res.0.len(), file);

    Ok(res)
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
