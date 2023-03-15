use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::PackageType;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use super::parsers::yarn;
use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

pub struct PackageLock;
pub struct YarnLock;

impl Parse for PackageLock {
    /// Parses `package-lock.json` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
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
                // Discard version information of local packages.
                //
                // In NPM, versions for filesystem dependencies are in the object with the
                // `name` corresponding to the path of the module, without any mention of the
                // module's name itself.
                //
                // The module's name then shows up as a separate package with the `name` as
                // `node_modules/<NAME>`, the path as `resolved`, `"link": true` and no version.
                //
                // Since we care more about the name of a local dependency than its package, we
                // discard the version here and include the package later when it's mentioned by
                // name.
                let name = match name.rsplit_once("node_modules/") {
                    Some((_, name)) => name,
                    None => continue,
                };

                // Ignore bundled dependencies.
                if keys.get("inBundle").is_some() {
                    continue;
                }

                // Get dependency type.
                let resolved = get_field(keys, "resolved")
                    .ok_or_else(|| anyhow!("Dependency '{name}' is missing \"resolved\" key"))?;

                // Get dependency version.
                let version = if resolved.starts_with("https://registry.npmjs.org/") {
                    PackageVersion::FirstParty(get_version(keys, name)?)
                } else if resolved.starts_with("git+") {
                    PackageVersion::Git(resolved)
                } else if resolved.starts_with("http") {
                    // Split off `http(s)://`.
                    let mut split = resolved.split('/');
                    let _ = split.next();
                    let _ = split.next();

                    // Find registry's domain name.
                    match split.next() {
                        Some(registry) => PackageVersion::ThirdParty(ThirdPartyVersion {
                            version: get_version(keys, name)?,
                            registry: registry.into(),
                        }),
                        None => {
                            return Err(anyhow!("Invalid third party registry: {:?}", resolved));
                        },
                    }
                } else {
                    PackageVersion::Path(Some(resolved.into()))
                };

                packages.push(Package {
                    version,
                    name: name.into(),
                    package_type: PackageType::Npm,
                });
            }
            Ok(packages)
        } else if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_object()) {
            // Parser for package-lock.json <= v6.

            deps.into_iter()
                .map(|(name, keys)| {
                    Ok(Package {
                        version: PackageVersion::FirstParty(get_version(keys, name)?),
                        name: name.into(),
                        package_type: PackageType::Npm,
                    })
                })
                .collect()
        } else {
            Err(anyhow!("Failed to find dependencies"))
        }
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("package-lock.json"))
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
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
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

            let version = if resolver.starts_with("workspace:")
                || resolver.starts_with("file:")
                || resolver.starts_with("link:")
            {
                // Ignore project itself.
                if resolver == "workspace:." {
                    continue;
                }

                PackageVersion::Path(None)
            } else if resolver.starts_with("npm:") {
                let version = package
                    .get(&"version".to_string())
                    .and_then(YamlValue::as_str)
                    .ok_or_else(|| anyhow!("Failed to parse yarn version for '{}'", resolution))?;

                PackageVersion::FirstParty(version.into())
            } else if resolver.starts_with("http:")
                || resolver.starts_with("https:")
                || resolver.starts_with("ssh:")
            {
                if resolver.contains("#commit=") {
                    PackageVersion::Git(resolver)
                } else {
                    PackageVersion::DownloadUrl(resolver)
                }
            } else {
                return Err(anyhow!(
                    "Failed to parse yarn dependency resolver for '{}'",
                    resolution
                ));
            };

            packages.push(Package {
                name: name.to_owned(),
                version,
                package_type: PackageType::Npm,
            });
        }

        Ok(packages)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("yarn.lock"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_package() {
        let pkgs =
            PackageLock.parse(include_str!("../../tests/fixtures/package-lock-v6.json")).unwrap();

        assert_eq!(pkgs.len(), 17);
        assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
        assert_eq!(pkgs[0].version, PackageVersion::FirstParty("1.1.0".into()));

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "yargs-parser");
        assert_eq!(last.version, PackageVersion::FirstParty("20.2.4".into()));
    }

    #[test]
    fn lock_parse_package_v7() {
        let pkgs =
            PackageLock.parse(include_str!("../../tests/fixtures/package-lock.json")).unwrap();

        assert_eq!(pkgs.len(), 54);

        let expected_pkgs = [
            Package {
                name: "accepts".into(),
                version: PackageVersion::FirstParty("1.3.8".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "vary".into(),
                version: PackageVersion::FirstParty("1.1.2".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "typescript".into(),
                version: PackageVersion::Git(
                    "git+ssh://git@github.com/Microsoft/TypeScript.git#\
                     9189e42b1c8b1a91906a245a24697da5e0c11a08"
                        .into(),
                ),
                package_type: PackageType::Npm,
            },
            Package {
                name: "form-data".into(),
                version: PackageVersion::FirstParty("2.3.3".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "match-sorter".into(),
                version: PackageVersion::ThirdParty(ThirdPartyVersion {
                    registry: "custom-registry.org".into(),
                    version: "3.1.1".into(),
                }),
                package_type: PackageType::Npm,
            },
            Package {
                name: "test".into(),
                version: PackageVersion::Path(Some("../test".into())),
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
        let pkgs =
            YarnLock.parse(include_str!("../../tests/fixtures/yarn-v1.simple.lock")).unwrap();

        assert_eq!(pkgs, vec![Package {
            name: "@yarnpkg/lockfile".to_string(),
            version: PackageVersion::FirstParty("1.1.0".into()),
            package_type: PackageType::Npm,
        }]);
    }

    #[test]
    fn lock_parse_yarn_v1() {
        for p in [
            include_str!("../../tests/fixtures/yarn-v1.lock"),
            include_str!("../../tests/fixtures/yarn-v1.trailing_newlines.lock"),
        ] {
            let pkgs = YarnLock.parse(p).unwrap();

            assert_eq!(pkgs.len(), 17);

            assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
            assert_eq!(pkgs[0].version, PackageVersion::FirstParty("1.1.0".into()));

            assert_eq!(pkgs[3].name, "cliui");
            assert_eq!(pkgs[3].version, PackageVersion::FirstParty("7.0.4".into()));

            let last = pkgs.last().unwrap();
            assert_eq!(last.name, "yargs");
            assert_eq!(last.version, PackageVersion::FirstParty("16.2.0".into()));
        }
    }

    #[should_panic]
    #[test]
    fn lock_parse_yarn_v1_malformed_fails() {
        YarnLock.parse(include_str!("../../tests/fixtures/yarn-v1.lock.bad")).unwrap();
    }

    #[test]
    fn lock_parse_yarn() {
        let pkgs = YarnLock.parse(include_str!("../../tests/fixtures/yarn.lock")).unwrap();

        assert_eq!(pkgs.len(), 56);

        let expected_pkgs = [
            Package {
                name: "accepts".into(),
                version: PackageVersion::FirstParty("1.3.8".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "mime-types".into(),
                version: PackageVersion::FirstParty("2.1.35".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "statuses".into(),
                version: PackageVersion::FirstParty("1.5.0".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "@fake/package".into(),
                version: PackageVersion::FirstParty("1.2.3".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "ethereumjs-abi".into(),
                version: PackageVersion::Git("https://github.com/ethereumjs/ethereumjs-abi.git\
                    #commit=ee3994657fa7a427238e6ba92a84d0b529bbcde0"
                    .into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "@me/remote-patch".into(),
                version: PackageVersion::Git("ssh://git@github.com:phylum/remote-patch\
                    #commit=d854c43ea177d1faeea56189249fff8c24a764bd"
                    .into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "xxx".into(),
                version: PackageVersion::Path(None),
                package_type: PackageType::Npm,
            },
            Package {
                name: "testing".into(),
                version: PackageVersion::Path(None),
                package_type: PackageType::Npm,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
