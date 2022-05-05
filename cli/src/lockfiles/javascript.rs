use std::io;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
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
                .ok_or_else(|| anyhow!("Failed to parse version for '{}' dependency", name))?;
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
            Err(anyhow!("Failed to find dependencies"))
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
                let data = self.0.as_str();
                let (_, entries) = yarn::parse(data)
                    .finish()
                    .map_err(|e| anyhow!(convert_error(data, e)))
                    .context("Failed to parse yarn lock file")?;
                return Ok(entries);
            }
        };

        let mapping = yaml_v2
            .as_mapping()
            .ok_or_else(|| anyhow!("Invalid yarn v2 lock file"))?;

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
                .ok_or_else(|| anyhow!("Failed to parse resolution field in yarn lock file"))?;

            let (name, mut resolver) = match resolution[1..].split_once('@') {
                Some((name, resolver)) => (&resolution[..name.len() + 1], resolver.to_owned()),
                None => return Err(anyhow!("Failed to parse yarn depenency resolution")),
            };

            // Extract original resolver from patch.
            if let Some((_, patch)) = resolver.split_once("patch:") {
                // Exctract resolver from `@scope/package@RESOLVER#patch`.
                let patch = patch[1..].split_once('@');
                let subresolver = patch.and_then(|(_, resolver)| resolver.split_once('#'));
                resolver = match subresolver {
                    Some((resolver, _)) => resolver.to_owned(),
                    None => return Err(anyhow!("Failed to parse yarn patch dependency")),
                };

                // Revert character replacements.
                resolver = resolver.replace("%3A", ":");
                resolver = resolver.replace("%23", "#");
                resolver = resolver.replace("%25", "%");
            }

            let (name, version) = if resolver.starts_with("workspace:") {
                // Ignore filesystem dependencies like the project ("project@workspace:.").
                continue;
            } else if resolver.starts_with("npm:") {
                let version = package
                    .get(&"version".into())
                    .and_then(YamlValue::as_str)
                    .ok_or_else(|| anyhow!("Failed to parse version in yarn lock file"))?;

                (name, version.to_owned())
            } else if resolver.starts_with("http:")
                || resolver.starts_with("https:")
                || resolver.starts_with("ssh:")
            {
                (name, resolver)
            } else {
                return Err(anyhow!(
                    "Failed to parse yarn dependency resolver: {}",
                    resolver
                ));
            };

            packages.push(PackageDescriptor {
                package_type: PackageType::Npm,
                name: name.to_owned(),
                version,
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

        assert_eq!(pkgs.len(), 53);

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
            PackageDescriptor {
                name: "ethereumjs-abi".into(),
                version: "https://github.com/ethereumjs/ethereumjs-abi.git\
                    #commit=ee3994657fa7a427238e6ba92a84d0b529bbcde0"
                    .into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "@me/remote-patch".into(),
                version: "ssh://git@github.com:phylum/remote-patch\
                    #commit=d854c43ea177d1faeea56189249fff8c24a764bd"
                    .into(),
                package_type: PackageType::Npm,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
