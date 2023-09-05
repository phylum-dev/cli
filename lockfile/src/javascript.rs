use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
#[cfg(feature = "generator")]
use lockfile_generator::npm::Npm as NpmGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::pnpm::Pnpm as PnpmGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::yarn::Yarn as YarnGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::PackageType;
use serde::Deserialize;
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
                if !name.starts_with("node_modules/") {
                    continue;
                }

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
        let file_name = path.file_name();
        file_name == Some(OsStr::new("package-lock.json"))
            || file_name == Some(OsStr::new("npm-shrinkwrap.json"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("package.json"))
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&NpmGenerator)
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
                    .context("Failed to parse yarn lockfile")?;
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

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("package.json"))
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&YarnGenerator)
    }
}

pub struct Pnpm;

impl Parse for Pnpm {
    /// Parses `pnpm-lock.yaml` files into a vec of packages.
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let lockfile: PnpmLock = serde_yaml::from_str(data)?;
        lockfile.packages()
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("pnpm-lock.yaml"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("package.json"))
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&PnpmGenerator)
    }
}

/// `pnpm-lock.yaml` structure.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PnpmLock {
    #[serde(rename = "lockfileVersion")]
    _lockfile_version: String,
    #[serde(default)]
    packages: HashMap<String, PnpmPackage>,
}

impl PnpmLock {
    /// Get all packages in the lockfile.
    fn packages(self) -> anyhow::Result<Vec<Package>> {
        let mut packages = Vec::new();

        for (name, package) in self.packages.into_iter() {
            // Parse package based on available fields.
            let tarball = package.resolution.tarball;
            let git = package.resolution.repo.zip(package.resolution.commit);
            let package = match (tarball, git) {
                (Some(tarball), _) => Self::tarball_package(tarball, package.name)?,
                (_, Some((repo, commit))) => Self::git_package(repo, commit, package.name)?,
                _ => Self::firstparty_package(&name)?,
            };

            packages.push(package);
        }

        Ok(packages)
    }

    /// Parse a first-party registry package.
    fn firstparty_package(name: &str) -> anyhow::Result<Package> {
        // Remove `/` prefix.
        let name = name
            .strip_prefix('/')
            .ok_or_else(|| anyhow!("Dependency '{name}' is missing '/' prefix"))?;

        // Remove annotations.
        let name = name.split_once('(').map(|(name, _)| name).unwrap_or(name);

        // Separate name and version.
        match name.rsplit_once('@') {
            Some((name, version)) => Ok(Package {
                name: name.into(),
                version: PackageVersion::FirstParty(version.into()),
                package_type: PackageType::Npm,
            }),
            None => Err(anyhow!("Dependency '{name}' is missing a version")),
        }
    }

    /// Parse a tarball package.
    fn tarball_package(tarball: String, name: Option<String>) -> anyhow::Result<Package> {
        let name = name.ok_or_else(|| anyhow!("Tarball '{tarball}' is missing a name"))?;

        Ok(Package {
            name,
            version: PackageVersion::DownloadUrl(tarball),
            package_type: PackageType::Npm,
        })
    }

    /// Parse a git package.
    fn git_package(repo: String, commit: String, name: Option<String>) -> anyhow::Result<Package> {
        let name = name.ok_or_else(|| anyhow!("Tarball '{repo}#{commit}' is missing a name"))?;
        let git_uri = format!("{repo}#{commit}");

        Ok(Package { name, version: PackageVersion::Git(git_uri), package_type: PackageType::Npm })
    }
}

/// `pnpm-lock.yaml` package structure.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PnpmPackage {
    resolution: PnpmResolution,
    name: Option<String>,
}

/// `pnpm-lock.yaml` resolution structure.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PnpmResolution {
    tarball: Option<String>,
    commit: Option<String>,
    repo: Option<String>,
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

        assert_eq!(pkgs.len(), 55);

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
            Package {
                name: "parentlink".into(),
                version: PackageVersion::Path(Some("../node_modules/parentlink".into())),
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

        assert_eq!(
            pkgs,
            vec![Package {
                name: "@yarnpkg/lockfile".to_string(),
                version: PackageVersion::FirstParty("1.1.0".into()),
                package_type: PackageType::Npm,
            }]
        );
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
                version: PackageVersion::Git(
                    "https://github.com/ethereumjs/ethereumjs-abi.git\
                    #commit=ee3994657fa7a427238e6ba92a84d0b529bbcde0"
                        .into(),
                ),
                package_type: PackageType::Npm,
            },
            Package {
                name: "@me/remote-patch".into(),
                version: PackageVersion::Git(
                    "ssh://git@github.com:phylum/remote-patch\
                    #commit=d854c43ea177d1faeea56189249fff8c24a764bd"
                        .into(),
                ),
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

    #[test]
    fn empty_yarn_v1() {
        let pkgs = YarnLock.parse(include_str!("../../tests/fixtures/yarn-v1.empty.lock")).unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn empty_yarn_v2() {
        // While this uses the same parser as the `empty_yarn_v1` test, this should make
        // sure we do not accidentally introduce a regression if we ever remove the v1
        // parser.
        let pkgs = YarnLock.parse("").unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn pnpm() {
        let pkgs = Pnpm.parse(include_str!("../../tests/fixtures/pnpm-lock.yaml")).unwrap();

        assert_eq!(pkgs.len(), 64);

        let expected_pkgs = [
            Package {
                name: "accepts".into(),
                version: PackageVersion::FirstParty("1.3.8".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "bootstrap".into(),
                version: PackageVersion::FirstParty("5.3.0".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "@babel/core".into(),
                version: PackageVersion::FirstParty("7.22.5".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "bytes".into(),
                version: PackageVersion::FirstParty("1.2.3-rc4".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "typescript".into(),
                version: PackageVersion::DownloadUrl("https://codeload.github.com/Microsoft/TypeScript/tar.gz/a437de66b6d6f36f205eafcd21a732a29f905486".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "demo".into(),
                version: PackageVersion::DownloadUrl("https://gitlab.com/api/v4/projects/Phylum%2demo/repository/archive.tar.gz?ref=ab3010efa019564710a03010abace10afeb0a2fe".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "testing".into(),
                version: PackageVersion::Git("ssh://git@git.sr.ht/~undeadleech/pnpm-test#cf066e8d69df5ba2cf3d4275b9e775800148d7ff".into()),
                package_type: PackageType::Npm,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg), "missing package {expected_pkg:?}");
        }
    }

    #[test]
    fn empty_pnpm() {
        let lockfile = "lockfileVersion: '6.0'\n\nsettings:\n  autoInstallPeers: true\n  \
                        excludeLinksFromLockfile: false\n";
        let pkgs = Pnpm.parse(lockfile).unwrap();

        assert!(pkgs.is_empty());
    }
}
