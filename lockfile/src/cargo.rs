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
    metadata: CargoMetadata,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    version: String,
    source: Option<PackageSource>,
}

#[derive(Deserialize, Debug)]
struct PackageSource {
    #[serde(rename = "type")]
    source_type: String,
    url: String,
    resolved_reference: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct CargoMetadata {
    lock_version: String,
}

impl Parse for Cargo {
    /// Parses `cargo.lock` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let mut lock: CargoLock = toml::from_str(data)?;

        // Warn if the version of this lockfile might not be supported.
        if !lock.metadata.lock_version.starts_with("1.") {
            log::debug!(
                "Expected cargo lockfile version ^1.0.0, found {}.",
                lock.metadata.lock_version
            );
        }

        Ok(lock
            .packages
            .drain(..)
            .filter(|package| {
                package.source.as_ref().map_or(true, |source| {
                    source.source_type == "git" || source.source_type == "legacy"
                })
            })
            .map(PackageDescriptor::from)
            .collect())
    }

    fn package_type(&self) -> PackageType {
        PackageType::Cargo
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("cargo.lock"))
    }
}

impl From<Package> for PackageDescriptor {
    fn from(package: Package) -> Self {
        let version = package
            .source
            .and_then(|source| {
                let reference = source.resolved_reference.as_ref();
                reference.map(|reference| format!("{}#{}", source.url, reference))
            })
            .unwrap_or(package.version);

        Self { name: package.name, package_type: PackageType::Cargo, version }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cargo_lock() {
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/rust-cargo.lock")).unwrap();
        assert_eq!(pkgs.len(), 45);

        let expected_pkgs = [
            PackageDescriptor {
                name: "cachecontrol".into(),
                version: "0.12.10".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "flask".into(),
                version: "2.1.1".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "cargo".into(),
                version: "https://github.com/python-cargo/cargo.git#4bc181b06ff9780791bc9e3d5b11bb807ca29d70".into(),
                package_type: PackageType::Cargo,
            },
            PackageDescriptor {
                name: "autopep8".into(),
                version: "1.5.6".into(),
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
        let pkgs = Cargo.parse(include_str!("../../tests/fixtures/rust-cargo.lock")).unwrap();

        let invalid_package_names = ["toml", "directory-test", "requests"];
        for pkg in pkgs {
            assert!(!invalid_package_names.contains(&pkg.name.as_str()));
        }
    }
}
