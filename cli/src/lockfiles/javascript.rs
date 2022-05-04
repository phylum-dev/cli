use std::io;
use std::path::Path;

use phylum_types::types::package::{PackageDescriptor, PackageType};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use super::parsers::yarn;
use crate::lockfiles::{ParseResult, Parseable};

pub struct PackageLock(String);
pub struct YarnLock(String);

impl Parseable for PackageLock {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(PackageLock(std::fs::read_to_string(filename)?))
    }

    /// Parses `package-lock.json` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let parsed: JsonValue = serde_json::from_str(&self.0)?;

        let into_descriptor = |(name, v): (String, &JsonValue)| {
            let version = v
                .as_object()
                .and_then(|x| x.get("version"))
                .and_then(|v| v.as_str())
                .map(|x| x.to_string())
                .ok_or_else(|| format!("Failed to parse version for '{}' dependency", name))?;
            let pkg = PackageDescriptor {
                name,
                version,
                package_type: PackageType::Npm,
            };
            Ok(pkg)
        };

        if let Some(deps) = parsed.get("packages").and_then(|v| v.as_object()) {
            deps.into_iter()
                // Ignore empty reference to package itself.
                .filter(|(k, _v)| !k.is_empty())
                // Get module name from path.
                .map(|(k, v)| {
                    let module = k.rsplit_once("node_modules/").map(|(_, k)| k).unwrap_or(k);
                    (module.to_owned(), v)
                })
                .map(into_descriptor)
                .collect()
        } else if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_object()) {
            deps.into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .map(into_descriptor)
                .collect()
        } else {
            Err("Failed to find dependencies".into())
        }
    }
}

impl Parseable for YarnLock {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(YarnLock(std::fs::read_to_string(filename)?))
    }

    /// Parses `yarn.lock` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let yaml_v2: YamlValue = match serde_yaml::from_str(&self.0) {
            Ok(yaml) => yaml,
            Err(_) => {
                let (_, entries) =
                    yarn::parse(&self.0).map_err(|_e| "Failed to parse yarn lock file")?;
                return Ok(entries);
            }
        };

        let mapping = yaml_v2.as_mapping().ok_or("Invalid yarn v2 lock file")?;

        let mut packages = Vec::new();
        for package in mapping
            .iter()
            // Filter lockfile data fields like "__metadata".
            .filter(|(k, _v)| k.as_str().map_or(false, |k| !k.starts_with('_')))
            .flat_map(|(_k, v)| v.as_mapping())
        {
            let resolution = package
                .get(&"resolution".into())
                .and_then(YamlValue::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Failed to parse resolution field in yarn lock file".to_owned())?;

            // Ignore workspace-local dependencies like project itself ("project@workspace:.").
            if resolution[1..].contains("@workspace:") {
                continue;
            }

            let (name, _version) = if let Some(index) = resolution[1..].find("@patch:") {
                // Extract npm version from patched dependencies.
                resolution[index + "@patch:".len() + 1..]
                    .rsplit_once("@npm")
                    .ok_or_else(|| "Failed to parse patch in yarn lock file".to_owned())?
            } else {
                resolution
                    .rsplit_once("@npm")
                    .ok_or_else(|| "Failed to parse name in yarn lock file".to_owned())?
            };

            let version = package
                .get(&"version".into())
                .and_then(YamlValue::as_str)
                .ok_or_else(|| "Failed to parse version in yarn lock file".to_owned())?;

            packages.push(PackageDescriptor {
                name: name.to_owned(),
                version: version.to_owned(),
                package_type: PackageType::Npm,
            });
        }

        Ok(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_package() {
        let parser = PackageLock::new(Path::new("tests/fixtures/package-lock-v6.json")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 17);
        assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
        assert_eq!(pkgs[0].version, "1.1.0");
        assert_eq!(pkgs[0].package_type, PackageType::Npm);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "yargs-parser");
        assert_eq!(last.version, "20.2.4");
        assert_eq!(last.package_type, PackageType::Npm);
    }

    #[test]
    fn lock_parse_package_v7() {
        let parser = PackageLock::new(Path::new("tests/fixtures/package-lock.json")).unwrap();

        let pkgs = parser.parse().unwrap();

        assert_eq!(pkgs.len(), 50);

        let expected_pkgs = [
            PackageDescriptor {
                name: "accepts".into(),
                version: "1.3.8".into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "vary".into(),
                version: "1.1.2".into(),
                package_type: PackageType::Npm,
            },
        ];
        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn lock_parse_yarn_v1() {
        for p in &[
            "tests/fixtures/yarn-v1.lock",
            "tests/fixtures/yarn-v1.trailing_newlines.lock",
        ] {
            let parser = YarnLock::new(Path::new(p)).unwrap();

            let pkgs = parser.parse().unwrap();
            assert_eq!(pkgs.len(), 17);
            assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
            assert_eq!(pkgs[0].version, "1.1.0");
            assert_eq!(pkgs[0].package_type, PackageType::Npm);

            let last = pkgs.last().unwrap();
            assert_eq!(last.name, "yargs");
            assert_eq!(last.version, "16.2.0");
            assert_eq!(last.package_type, PackageType::Npm);
        }
    }

    #[should_panic]
    #[test]
    fn lock_parse_yarn_v1_malformed_fails() {
        let parser = YarnLock::new(Path::new("tests/fixtures/yarn-v1.lock.bad")).unwrap();

        parser.parse().unwrap();
    }

    #[test]
    fn lock_parse_yarn() {
        let parser = YarnLock::new(Path::new("tests/fixtures/yarn.lock")).unwrap();

        let pkgs = parser.parse().unwrap();

        assert_eq!(pkgs.len(), 51);

        let expected_pkgs = [
            PackageDescriptor {
                name: "accepts".into(),
                version: "1.3.8".into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "mime-types".into(),
                version: "2.1.35".into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "statuses".into(),
                version: "1.5.0".into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "@fake/package".into(),
                version: "1.2.3".into(),
                package_type: PackageType::Npm,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
