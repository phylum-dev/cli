use std::ffi::OsStr;
use std::path::Path;

use phylum_types::types::package::{PackageDescriptor, PackageType};
use serde::Deserialize;

use crate::{Parse, ParseResult};

pub struct Cargo;

#[derive(Deserialize, Debug)]
struct CargoLock {
    #[serde(rename = "package")]
    packages: Vec<Package>,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    version: String,
}

impl Parse for Cargo {
    /// Parses `cargo.lock` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let mut lock: CargoLock = toml::from_str(data).unwrap();
        Ok(lock.packages.drain(..).map(PackageDescriptor::from).collect())
    }

    fn package_type(&self) -> PackageType {
        println!("declared package type");
        PackageType::Cargo
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Cargo.lock"))
    }
}

impl From<Package> for PackageDescriptor {
    fn from(package: Package) -> Self {
        let version = package.version;

        Self { name: package.name, package_type: PackageType::Cargo, version }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cargo_lock() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo.lock")).unwrap();
        assert_eq!(pkgs.len(), 533);

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
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    /// Ensure sources other than Cargo are ignored.
    #[test]
    fn cargo_ignore_other_sources() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/Cargo.lock")).unwrap();

        let invalid_package_names = ["toml", "directory-test", "requests"];
        for pkg in pkgs {
            assert!(!invalid_package_names.contains(&pkg.name.as_str()));
        }
    }
}
