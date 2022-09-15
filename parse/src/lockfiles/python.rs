use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::{PackageDescriptor, PackageType};
use serde::Deserialize;
use serde_json::Value;

use super::parsers::pypi;
use crate::lockfiles::{LockfileFormat, Parse, ParseResult};

pub struct PyRequirements;
pub struct PipFile;
pub struct Poetry;

impl Parse for PyRequirements {
    /// Parses `requirements.txt` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let (_, entries) = pypi::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse requirements file")?;
        Ok(entries)
    }

    fn format(&self) -> LockfileFormat {
        LockfileFormat::Pip
    }

    fn package_type(&self) -> PackageType {
        PackageType::PyPi
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("requirements.txt"))
    }
}

impl Parse for PipFile {
    /// Parses `Pipfile` or `Pipfile.lock` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let mut input: HashMap<String, Value> = match toml::from_str::<toml::Value>(data).ok() {
            Some(s) => serde_json::from_value(serde_json::to_value(s)?)?,
            None => serde_json::from_str(data)?,
        };

        let mut packages: HashMap<String, Value> =
            serde_json::from_value(input.remove("packages").unwrap_or_default())
                .unwrap_or_default();
        let dev_packages: HashMap<String, Value> =
            serde_json::from_value(input.remove("dev-packages").unwrap_or_default())
                .unwrap_or_default();
        let default: HashMap<String, Value> =
            serde_json::from_value(input.remove("default").unwrap_or_default()).unwrap_or_default();
        let develop: HashMap<String, Value> =
            serde_json::from_value(input.remove("develop").unwrap_or_default()).unwrap_or_default();

        packages.extend(dev_packages);
        packages.extend(default);
        packages.extend(develop);

        packages
            .iter()
            .filter_map(|(k, v)| {
                let version = match v {
                    Value::String(s) if s.contains("==") => Some(v.as_str().unwrap_or_default()),
                    Value::Object(s) => match s.get("version") {
                        Some(s) if s.as_str().unwrap_or_default().contains("==") => {
                            Some(s.as_str().unwrap_or_default())
                        },
                        _ => None,
                    },
                    _ => None,
                };
                match version {
                    Some(_) => version.map(|v| {
                        Ok(PackageDescriptor {
                            name: k.as_str().to_string().to_lowercase(),
                            version: v.replace("==", "").trim().to_string(),
                            package_type: self.package_type(),
                        })
                    }),
                    None => {
                        log::debug!("Could not determine version for package: {}", k);
                        None
                    },
                }
            })
            .collect::<Result<Vec<_>, _>>()
    }

    fn format(&self) -> LockfileFormat {
        LockfileFormat::Pipenv
    }

    fn package_type(&self) -> PackageType {
        PackageType::PyPi
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Pipfile"))
            || path.file_name() == Some(OsStr::new("Pipfile.lock"))
    }
}

impl Parse for Poetry {
    /// Parses `poetry.lock` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let mut lock: PoetryLock = toml::from_str(data)?;

        // Warn if the version of this lockfile might not be supported.
        if !lock.metadata.lock_version.starts_with("1.") {
            log::debug!(
                "Expected poetry lockfile version ^1.0.0, found {}.",
                lock.metadata.lock_version
            );
        }

        Ok(lock
            .packages
            .drain(..)
            .filter(|package| {
                package.source.as_ref().map_or(true, |source| source.source_type == "git")
            })
            .map(PackageDescriptor::from)
            .collect())
    }

    fn format(&self) -> LockfileFormat {
        LockfileFormat::Poetry
    }

    fn package_type(&self) -> PackageType {
        PackageType::PyPi
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("poetry.lock"))
    }
}

#[derive(Deserialize, Debug)]
struct PoetryLock {
    #[serde(rename = "package")]
    packages: Vec<Package>,
    metadata: PoetryMetadata,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    version: String,
    source: Option<PackageSource>,
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

        Self { name: package.name, package_type: PackageType::PyPi, version }
    }
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
struct PoetryMetadata {
    lock_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requirements() {
        let pkgs =
            PyRequirements.parse(include_str!("../../../tests/fixtures/requirements.txt")).unwrap();
        assert_eq!(pkgs.len(), 131);
        assert_eq!(pkgs[0].name, "pyyaml");
        assert_eq!(pkgs[0].version, "5.4.1");
        assert_eq!(pkgs[0].package_type, PackageType::PyPi);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "zope.interface");
        assert_eq!(last.version, "5.4.0");
        assert_eq!(last.package_type, PackageType::PyPi);
    }

    #[test]
    fn parse_requirements_complex() {
        let pkgs = PyRequirements
            .parse(include_str!("../../../tests/fixtures/complex-requirements.txt"))
            .unwrap();
        assert_eq!(pkgs.len(), 8);
        assert_eq!(pkgs[0].name, "docopt");
        assert_eq!(pkgs[0].version, "0.6.1");
        assert_eq!(pkgs[0].package_type, PackageType::PyPi);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "git-for-pip-example");
        assert_eq!(
            last.version,
            "git+https://github.com/matiascodesal/git-for-pip-example.git@v1.0.0"
        );
        assert_eq!(last.package_type, PackageType::PyPi);
    }

    #[test]
    fn parse_pipfile() {
        let pkgs = PipFile.parse(include_str!("../../../tests/fixtures/Pipfile")).unwrap();
        assert_eq!(pkgs.len(), 4);

        let expected_pkgs = [
            PackageDescriptor {
                name: "pypresence".into(),
                version: "4.0.0".into(),
                package_type: PackageType::PyPi,
            },
            PackageDescriptor {
                name: "chromedriver-py".into(),
                version: "91.0.4472.19".into(),
                package_type: PackageType::PyPi,
            },
            PackageDescriptor {
                name: "requests".into(),
                version: "2.24.0".into(),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn lock_parse_pipfile() {
        let pkgs = PipFile.parse(include_str!("../../../tests/fixtures/Pipfile.lock")).unwrap();
        assert_eq!(pkgs.len(), 27);

        let expected_pkgs = [
            PackageDescriptor {
                name: "jdcal".into(),
                version: "1.3".into(),
                package_type: PackageType::PyPi,
            },
            PackageDescriptor {
                name: "certifi".into(),
                version: "2017.7.27.1".into(),
                package_type: PackageType::PyPi,
            },
            PackageDescriptor {
                name: "unittest2".into(),
                version: "1.1.0".into(),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_poetry_lock() {
        let pkgs = Poetry.parse(include_str!("../../../tests/fixtures/poetry.lock")).unwrap();
        assert_eq!(pkgs.len(), 44);

        let expected_pkgs = [
            PackageDescriptor {
                name: "cachecontrol".into(),
                version: "0.12.10".into(),
                package_type: PackageType::PyPi,
            },
            PackageDescriptor {
                name: "flask".into(),
                version: "2.1.1".into(),
                package_type: PackageType::PyPi,
            },
            PackageDescriptor {
                name: "poetry".into(),
                version: "https://github.com/python-poetry/poetry.git#4bc181b06ff9780791bc9e3d5b11bb807ca29d70".into(),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    /// Ensure sources other than PyPi are ignored.
    #[test]
    fn poetry_ignore_other_sources() {
        let pkgs = Poetry.parse(include_str!("../../../tests/fixtures/poetry.lock")).unwrap();

        let invalid_package_names = ["toml", "directory-test", "requests"];
        for pkg in pkgs {
            assert!(!invalid_package_names.contains(&pkg.name.as_str()));
        }
    }
}
