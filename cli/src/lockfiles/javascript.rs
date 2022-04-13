use serde_json::Value;
use std::io;
use std::path::Path;

use phylum_types::types::package::{PackageDescriptor, PackageType};

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
        let parsed: Value = serde_json::from_str(&self.0)?;

        let into_descriptor = |(name, v): (String, &Value)| {
            let pkg = PackageDescriptor {
                name,
                version: v
                    .as_object()
                    .and_then(|x| x["version"].as_str())
                    .map(|x| x.to_string())
                    .ok_or("Failed to parse version")?,
                package_type: PackageType::Npm,
            };
            Ok(pkg)
        };

        if let Some(deps) = parsed["packages"].as_object() {
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
        } else if let Some(deps) = parsed["dependencies"].as_object() {
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
        let (_, entries) = yarn::parse(&self.0).map_err(|_e| "Failed to parse yarn lock file")?;
        Ok(entries)
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
    fn lock_parse_yarn() {
        for p in &[
            "tests/fixtures/yarn.lock",
            "tests/fixtures/yarn.trailing_newlines.lock",
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
    fn lock_parse_yarn_malformed_fails() {
        let parser = YarnLock::new(Path::new("tests/fixtures/yarn.lock.bad")).unwrap();

        parser.parse().unwrap();
    }
}
