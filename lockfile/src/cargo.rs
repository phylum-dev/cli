use std::ffi::OsStr;
use std::path::Path;

use phylum_types::types::package::{PackageDescriptor, PackageType};
use serde::Deserialize;

use crate::{Parse, ParseResult};

#[derive(Deserialize, Debug, Clone)]
struct CargoLock {
    #[serde(rename = "package")]
    packages: Vec<Package>,
}

#[derive(Deserialize, Debug, Clone)]
struct Package {
    name: String,
    version: String,
    source: Option<String>,
}

pub struct Cargo;

impl Parse for Cargo {
    /// Parses `Cargo.lock` files into a vec of packages.
    fn parse(&self, data: &str) -> ParseResult {
        let mut lock: CargoLock = toml::from_str(data)?;
        Ok(lock
            .packages
            .drain(..)
            .filter_map(|package| PackageDescriptor::try_from(package).ok())
            .collect())
    }

    fn package_type(&self) -> PackageType {
        PackageType::Cargo
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Cargo.lock"))
    }
}

impl TryFrom<Package> for PackageDescriptor {
    type Error = ();

    fn try_from(package: Package) -> Result<Self, Self::Error> {
        let source = package.source.ok_or(())?;
        let version = if let Some(git_version) = source.strip_prefix("git+") {
            git_version.into()
        } else {
            package.version
        };

        Ok(Self { name: package.name, package_type: PackageType::Cargo, version })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cargo_lock_v1() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo_v1.lock")).unwrap();
        assert_eq!(pkgs.len(), 136);
        let expected_pkgs = [
            PackageDescriptor {
                name: "core-foundation".into(),
                version: "0.6.4".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "adler32".into(),
                version: "1.0.4".into(),
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
        assert_eq!(pkgs.len(), 24);

        let expected_pkgs = [PackageDescriptor {
            name: "form_urlencoded".into(),
            version: "1.0.1".into(),
            package_type: PackageType::Cargo,
        }];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
    #[test]
    fn parse_cargo_lock_v3() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo_v3.lock")).unwrap();
        assert_eq!(pkgs.len(), 530);

        let expected_pkgs = [
            PackageDescriptor {
                name: "Inflector".into(),
                version: "0.11.4".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "adler".into(),
                version: "1.0.2".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "aead".into(),
                version: "0.5.1".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "aes".into(),
                version: "0.8.1".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "landlock".into(),
                version: "https://github.com/phylum-dev/rust-landlock#b553736cefc2a740eda746e5730cf250b069a4c1".into(),
                package_type: PackageType::Cargo,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
    /// Ensure sources other than Cargo are ignored.
    #[test]
    fn cargo_ignore_other_sources() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo_v3.lock")).unwrap();

        let invalid_package_names = ["xtask", "phylum-cli", "phylum_lockfile"];
        for pkg in pkgs {
            assert!(!invalid_package_names.contains(&pkg.name.as_str()));
        }
    }
}
