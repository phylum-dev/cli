use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::{PackageDescriptor, PackageType};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use super::parsers::yarn;
use crate::lockfiles::{Parse, ParseResult};

pub struct PackageLock;
pub struct YarnLock;

impl Parse for PackageLock {
    /// Parses `package-lock.json` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let parsed: JsonValue = serde_json::from_str(data)?;

        // Get a field as string from a JSON object.
        let get_field = |value: &JsonValue, key| {
            value.get(key).and_then(|value| value.as_str()).map(|value| value.to_string())
        };

        // Get version field from JSON object.
        let get_version = |value, name| {
            get_field(value, "version")
                .ok_or_else(|| anyhow!("Failed to parse version for '{name}' dependency"))
        };

        if let Some(deps) = parsed.get("packages").and_then(|v| v.as_object()) {
            // Parser for package-lock.json >= v7.

            let mut packages = Vec::new();
            for (name, keys) in deps {
                // Ignore local filesystem dependencies.
                let name = match name.strip_prefix("node_modules/") {
                    Some(name) => name,
                    None => continue,
                };

                // Get dependency type.
                let resolved = get_field(keys, "resolved")
                    .ok_or_else(|| anyhow!("Dependency '{name}' is missing \"resolved\" key"))?;

                // Get dependency version.
                let version = if resolved.starts_with("https://registry.npmjs.org") {
                    get_version(keys, name)?
                } else if let Some(git_url) = resolved.strip_prefix("git+") {
                    git_url.to_owned()
                } else {
                    // Filter filesystem dependencies.
                    continue;
                };

                packages.push(PackageDescriptor {
                    version,
                    package_type: self.package_type(),
                    name: name.into(),
                });
            }
            Ok(packages)
        } else if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_object()) {
            // Parser for package-lock.json <= v6.

            deps.into_iter()
                .map(|(name, keys)| {
                    Ok(PackageDescriptor {
                        package_type: self.package_type(),
                        version: get_version(keys, name)?,
                        name: name.into(),
                    })
                })
                .collect()
        } else {
            Err(anyhow!("Failed to find dependencies"))
        }
    }

    fn package_type(&self) -> PackageType {
        PackageType::Npm
    }
}

/// Check if a YAML file is a valid v2 yarn lockfile.
///
/// Since some v1 yarn lockfiles can be parsed as valid yaml, this ensures that
/// the __metadata field is present to identify v2 lockfiles.
fn is_yarn_v2(yaml: &&serde_yaml::Mapping) -> bool {
    yaml.iter().any(|(k, _v)| k.as_str().unwrap_or_default() == "__metadata")
}

impl Parse for YarnLock {
    /// Parses `yarn.lock` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let yaml = serde_yaml::from_str::<YamlValue>(data).ok();
        let yaml_mapping = yaml.as_ref().and_then(|yaml| yaml.as_mapping());

        // Check if we should use v1 or v2 yarn parser.
        let yaml_v2 = match yaml_mapping.filter(is_yarn_v2) {
            Some(yaml_v2) => yaml_v2,
            _ => {
                let (_, entries) = yarn::parse(data)
                    .finish()
                    .map_err(|e| anyhow!(convert_error(data, e)))
                    .context("Failed to parse yarn lock file")?;
                return Ok(entries);
            },
        };

        let mut packages = Vec::new();
        for package in yaml_v2
            .iter()
            // Filter lockfile data fields like "__metadata".
            .filter(|(k, _v)| k.as_str().map_or(false, |k| !k.starts_with('_')))
            .flat_map(|(_k, v)| v.as_mapping())
        {
            let resolution = package
                .get(&"resolution".to_string())
                .and_then(YamlValue::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("Failed to parse yarn resolution field"))?;

            let (name, mut resolver) = match resolution[1..].split_once('@') {
                Some((name, resolver)) => (&resolution[..name.len() + 1], resolver.to_owned()),
                None => {
                    return Err(anyhow!(
                        "Failed to parse yarn resolution field for '{}'",
                        resolution
                    ))
                },
            };

            // Extract original resolver from patch.
            if let Some((_, patch)) = resolver.split_once("patch:") {
                // Exctract resolver from `@scope/package@RESOLVER#patch`.
                let patch = patch[1..].split_once('@');
                let subresolver = patch.and_then(|(_, resolver)| resolver.split_once('#'));
                resolver = match subresolver {
                    Some((resolver, _)) => resolver.to_owned(),
                    None => {
                        return Err(anyhow!(
                            "Failed to parse yarn patch dependency for '{}'",
                            resolution
                        ))
                    },
                };

                // Revert character replacements.
                resolver = resolver.replace("%3A", ":");
                resolver = resolver.replace("%23", "#");
                resolver = resolver.replace("%25", "%");
            }

            let (name, version) = if resolver.starts_with("workspace:")
                || resolver.starts_with("link:")
            {
                // Ignore filesystem dependencies like the project ("project@workspace:.").
                continue;
            } else if resolver.starts_with("npm:") {
                let version = package
                    .get(&"version".to_string())
                    .and_then(YamlValue::as_str)
                    .ok_or_else(|| anyhow!("Failed to parse yarn version for '{}'", resolution))?;

                (name, version.to_owned())
            } else if resolver.starts_with("http:")
                || resolver.starts_with("https:")
                || resolver.starts_with("ssh:")
            {
                (name, resolver)
            } else {
                return Err(anyhow!(
                    "Failed to parse yarn dependency resolver for '{}'",
                    resolution
                ));
            };

            packages.push(PackageDescriptor {
                package_type: self.package_type(),
                name: name.to_owned(),
                version,
            });
        }

        Ok(packages)
    }

    fn package_type(&self) -> PackageType {
        PackageType::Npm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_package() {
        let pkgs = PackageLock.parse_file("tests/fixtures/package-lock-v6.json").unwrap();

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
        let pkgs = PackageLock.parse_file("tests/fixtures/package-lock.json").unwrap();

        assert_eq!(pkgs.len(), 51);

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
            PackageDescriptor {
                name: "typescript".into(),
                version: "ssh://git@github.com/Microsoft/TypeScript.git#\
                          9189e42b1c8b1a91906a245a24697da5e0c11a08"
                    .into(),
                package_type: PackageType::Npm,
            },
        ];
        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn lock_parse_yarn_v1_simple() {
        // This file contains only one package and that package has no dependencies.
        // This makes the file valid YAML according to serde_yaml.
        //
        // We need to make sure we don't take the v2 lockfile code path because this is
        // not a v2 lockfile and parsing it as one will produce incorrect
        // results.
        let pkgs = YarnLock.parse_file("tests/fixtures/yarn-v1.simple.lock").unwrap();

        assert_eq!(pkgs, vec![PackageDescriptor {
            name: "@yarnpkg/lockfile".to_string(),
            version: "1.1.0".to_string(),
            package_type: PackageType::Npm,
        }]);
    }

    #[test]
    fn lock_parse_yarn_v1() {
        for p in &["tests/fixtures/yarn-v1.lock", "tests/fixtures/yarn-v1.trailing_newlines.lock"] {
            let pkgs = YarnLock.parse_file(p).unwrap();

            assert_eq!(pkgs.len(), 17);

            assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
            assert_eq!(pkgs[0].version, "1.1.0");
            assert_eq!(pkgs[0].package_type, PackageType::Npm);

            assert_eq!(pkgs[3].name, "cliui");
            assert_eq!(pkgs[3].version, "7.0.4");
            assert_eq!(pkgs[3].package_type, PackageType::Npm);

            let last = pkgs.last().unwrap();
            assert_eq!(last.name, "yargs");
            assert_eq!(last.version, "16.2.0");
            assert_eq!(last.package_type, PackageType::Npm);
        }
    }

    #[should_panic]
    #[test]
    fn lock_parse_yarn_v1_malformed_fails() {
        YarnLock.parse_file("tests/fixtures/yarn-v1.lock.bad").unwrap();
    }

    #[test]
    fn lock_parse_yarn() {
        let pkgs = YarnLock.parse_file("tests/fixtures/yarn.lock").unwrap();

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
