use phylum_types::types::package::*;
use std::path::Path;

use crate::lockfiles::*;

/// Attempt to get packages from an unknown lockfile type
pub fn try_get_packages(path: &Path) -> Option<(Vec<PackageDescriptor>, PackageType)> {
    log::warn!(
        "Attempting to obtain packages from unrecognized lockfile type: {}",
        path.to_string_lossy()
    );

    let packages = YarnLock::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type yarn lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Npm));
    }

    let packages = PackageLock::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type package lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Npm));
    }

    let packages = GemLock::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type gem lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::RubyGems));
    }

    let packages = PyRequirements::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type pip requirements.txt");
        return packages.ok().map(|pkgs| (pkgs, PackageType::PyPi));
    }

    let packages = PipFile::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type pip Pipfile or Pipfile.lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::PyPi));
    }

    let packages = Poetry::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type poetry lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::PyPi));
    }

    let packages = Pom::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type pom xml");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Maven));
    }

    let packages = GradleDeps::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type gradle dependencies");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Maven));
    }

    let packages = CSProj::new(path).ok()?.parse();
    if packages.is_ok() && !packages.as_ref().unwrap().is_empty() {
        log::debug!("Submitting file as type csproj");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Nuget));
    }

    log::error!("Failed to identify lock file type");
    None
}

/// Determine the lockfile type based on its name and parse
/// accordingly to obtain the packages from it
pub fn get_packages_from_lockfile(path: &str) -> Option<(Vec<PackageDescriptor>, PackageType)> {
    let path = Path::new(path);
    let file = path.file_name()?.to_str()?;
    let ext = path.extension().and_then(|ext| ext.to_str());

    let pattern = match ext {
        Some("csproj") => ".csproj",
        _ => file,
    };

    let res = match pattern {
        "Gemfile.lock" => {
            let parser = GemLock::new(path).ok()?;
            parser
                .parse()
                .ok()
                .map(|pkgs| (pkgs, PackageType::RubyGems))
        }
        "package-lock.json" => {
            let parser = PackageLock::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Npm))
        }
        "yarn.lock" => {
            let parser = YarnLock::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Npm))
        }
        "requirements.txt" => {
            let parser = PyRequirements::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::PyPi))
        }
        "Pipfile.txt" | "Pipfile.lock" => {
            let parser = PipFile::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::PyPi))
        }
        "poetry.lock" => {
            let parser = Poetry::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::PyPi))
        }
        "effective-pom.xml" => {
            let parser = Pom::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Maven))
        }
        "gradle-dependencies.txt" => {
            let parser = GradleDeps::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Maven))
        }
        ".csproj" => {
            let parser = CSProj::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Nuget))
        }
        _ => try_get_packages(path),
    };

    let pkg_count = res.as_ref().map(|p| p.0.len()).unwrap_or_default();

    log::debug!("Read {} packages from file `{}`", pkg_count, file);

    res
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
