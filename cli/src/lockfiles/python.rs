use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

use phylum_types::types::package::{PackageDescriptor, PackageType};
use serde::Deserialize;
use serde_json::Value;

use super::parsers::pypi;
use crate::lockfiles::{ParseResult, Parseable};

pub struct PyRequirements(String);
pub struct PipFile(String);
pub struct Poetry(String);

impl Parseable for PyRequirements {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(PyRequirements(fs::read_to_string(filename)?))
    }

    /// Parses `requirements.txt` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let (_, entries) =
            pypi::parse(&self.0).map_err(|_e| "Failed to parse requirements file")?;
        Ok(entries)
    }
}

impl Parseable for PipFile {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(PipFile(fs::read_to_string(filename)?))
    }

    /// Parses `Pipfile` or `Pipfile.lock` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let mut input: HashMap<String, Value> = match toml::from_str::<toml::Value>(&self.0).ok() {
            Some(s) => serde_json::from_value(serde_json::to_value(s)?)?,
            None => serde_json::from_str(&self.0)?,
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
                        }
                        _ => None,
                    },
                    _ => None,
                };
                match version {
                    Some(_) => version.map(|v| {
                        Ok(PackageDescriptor {
                            name: k.as_str().to_string().to_lowercase(),
                            version: v.replace("==", "").trim().to_string(),
                            package_type: PackageType::PyPi,
                        })
                    }),
                    None => {
                        log::warn!("Could not determine version for package: {}", k);
                        None
                    }
                }
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

impl Parseable for Poetry {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(Poetry(fs::read_to_string(filename)?))
    }

    /// Parses `poetry.lock` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let mut lock: PoetryLock = toml::from_str(&self.0)?;

        // Warn if the version of this lockfile might not be supported.
        if !lock.metadata.lock_version.starts_with("1.") {
            log::warn!(
                "Expected poetry lockfile version ^1.0.0, found {}. \
                Attempting to continue, but results might be inaccurate.",
                lock.metadata.lock_version
            );
        }

        Ok(lock.packages.drain(..).map(Package::into).collect())
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
}

impl From<Package> for PackageDescriptor {
    fn from(package: Package) -> Self {
        Self {
            name: package.name,
            version: package.version,
            package_type: PackageType::PyPi,
        }
    }
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
        let parser = PyRequirements::new(Path::new("tests/fixtures/requirements.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 130);
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
        let parser =
            PyRequirements::new(Path::new("tests/fixtures/complex-requirements.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
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
        let parser = PipFile::new(Path::new("tests/fixtures/Pipfile")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 4);

        for pkg in &pkgs {
            if pkg.name == "pypresence" {
                assert_eq!(pkg.version, "4.0.0");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            } else if pkg.name == "chromedriver-py" {
                assert_eq!(pkg.version, "91.0.4472.19");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            } else if pkg.name == "requests" {
                assert_eq!(pkg.version, "2.24.0");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            }
        }
    }

    #[test]
    fn lock_parse_pipfile() {
        let parser = PipFile::new(Path::new("tests/fixtures/Pipfile.lock")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 27);

        for pkg in &pkgs {
            if pkg.name == "jdcal" {
                assert_eq!(pkg.version, "1.3");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            } else if pkg.name == "certifi" {
                assert_eq!(pkg.version, "2017.7.27.1");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            } else if pkg.name == "unittest2" {
                assert_eq!(pkg.version, "1.1.0");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            }
        }
    }

    #[test]
    fn parse_poetry_lock() {
        let parser = Poetry::new(Path::new("tests/fixtures/poetry.lock")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 62);

        for pkg in &pkgs {
            if pkg.name == "toml" {
                assert_eq!(pkg.version, "0.10.2");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            } else if pkg.name == "certifi" {
                assert_eq!(pkg.version, "2021.10.8");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            } else if pkg.name == "html5lib" {
                assert_eq!(pkg.version, "1.1");
                assert_eq!(pkg.package_type, PackageType::PyPi);
            }
        }
    }
}
