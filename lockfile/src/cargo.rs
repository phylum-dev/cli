use std::ffi::OsStr;
use std::path::Path;

use anyhow::anyhow;
#[cfg(feature = "generator")]
use lockfile_generator::cargo::Cargo as CargoGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use phylum_types::types::package::PackageType;
use serde::Deserialize;

use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

/// Default cargo registry URI.
const CARGO_REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Deserialize, Debug, Clone)]
struct CargoLock {
    #[serde(rename = "package")]
    packages: Vec<CargoPackage>,

    // NOTE: This is used to try and parse the lockfile as a Poetry and Cargo manifest
    // simultaneously, since both use toml with a list of [[package]].
    //
    // Everything in a minimal Cargo lockfile is also found in a Poetry lockfile, so we instead use
    // data found only in a Poetry lockfile to detect an invalid lockfile.
    //
    // We need to actually parse a field from the metadata struct since early versions of Cargo
    // lockfiles used it for hashes.
    #[serde(rename = "metadata")]
    python_metadata: Option<PoetryMetadata>,
}

#[derive(Deserialize, Debug, Clone)]
struct CargoPackage {
    name: String,
    version: String,
    source: Option<String>,
}

/// Metadata field of Poetry's lockfile.
#[derive(Deserialize, Debug, Clone)]
struct PoetryMetadata {
    #[serde(rename = "python-versions")]
    python_version: Option<String>,
}

pub struct Cargo;

impl Parse for Cargo {
    /// Parse a `Cargo.lock` file into an array of packages.
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let mut lock: CargoLock = toml::from_str(data)?;

        // Abort if we identified this as a Poetry lockfile.
        if lock.python_metadata.and_then(|metadata| metadata.python_version).is_some() {
            return Err(anyhow!("Cannot parse Poetry lockfile with Cargo.lock parser"));
        }

        lock.packages
            .drain(..)
            .map(|package| {
                let source = match package.source {
                    Some(source) => source,
                    // No package source means it's a local dependency.
                    None => {
                        return Ok(Package {
                            name: package.name,
                            version: PackageVersion::Path(None),
                            package_type: PackageType::Cargo,
                        })
                    },
                };

                let version = if source == CARGO_REGISTRY {
                    PackageVersion::FirstParty(package.version)
                } else if let Some(registry) = source.strip_prefix("registry+") {
                    PackageVersion::ThirdParty(ThirdPartyVersion {
                        registry: registry.into(),
                        version: package.version,
                    })
                } else if source.starts_with("git+") {
                    PackageVersion::Git(source)
                } else {
                    return Err(anyhow!(format!("Unknown cargo package source: {source:?}")));
                };

                Ok(Package { name: package.name, version, package_type: PackageType::Cargo })
            })
            .collect()
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Cargo.lock"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Cargo.toml"))
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&CargoGenerator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cargo_lock_v1() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo_v1.lock")).unwrap();
        assert_eq!(pkgs.len(), 141);
        let expected_pkgs = [
            Package {
                name: "core-foundation".into(),
                version: PackageVersion::FirstParty("0.6.4".into()),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "adler32".into(),
                version: PackageVersion::FirstParty("1.0.4".into()),
                package_type: PackageType::Cargo,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_cargo_lock_v2() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo_v2.lock")).unwrap();
        assert_eq!(pkgs.len(), 25);

        let expected_pkgs = [Package {
            name: "form_urlencoded".into(),
            version: PackageVersion::FirstParty("1.0.1".into()),
            package_type: PackageType::Cargo,
        }];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_cargo_lock_v3() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo_v3.lock")).unwrap();
        assert_eq!(pkgs.len(), 533);

        let expected_pkgs = [
            Package {
                name: "Inflector".into(),
                version: PackageVersion::FirstParty("0.11.4".into()),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "adler".into(),
                version: PackageVersion::FirstParty("1.0.2".into()),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "aead".into(),
                version: PackageVersion::FirstParty("0.5.1".into()),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "aes".into(),
                version: PackageVersion::FirstParty("0.8.1".into()),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "landlock".into(),
                version: PackageVersion::Git("git+https://github.com/phylum-dev/rust-landlock#b553736cefc2a740eda746e5730cf250b069a4c1".into()),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "xtask".into(),
                version: PackageVersion::Path(None),
                package_type: PackageType::Cargo,
            },
            Package {
                name: "zstd-sys".into(),
                version: PackageVersion::ThirdParty(ThirdPartyVersion {
                    registry: "https://invalid.veracode.com/foreign-registry-example".into(),
                    version: "1.6.3+zstd.1.5.2".into(),
                }),
                package_type: PackageType::Cargo,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
