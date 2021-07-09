use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::marker::Sized;
use std::path::Path;

use serde_json::Value;

use crate::types::{PackageDescriptor, PackageType};

mod parsers;
use parsers::{gem, pypi, yarn};

pub struct PackageLock(String);
pub struct YarnLock(String);
pub struct GemLock(String);
pub struct PyRequirements(String);
pub struct PipFile(String);

pub type ParseResult = Result<Vec<PackageDescriptor>, Box<dyn Error>>;

pub trait Parseable {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized;
    fn parse(&self) -> ParseResult;
}

impl Parseable for PackageLock {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(PackageLock(std::fs::read_to_string(filename)?))
    }

    /// Parses `package-lock.json` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let parsed: Value = serde_json::from_str(&self.0)?;

        parsed["dependencies"]
            .as_object()
            .ok_or("Failed to find dependencies")?
            .into_iter()
            .map(|(k, v)| {
                let pkg = PackageDescriptor {
                    name: k.as_str().to_string(),
                    version: v
                        .as_object()
                        .and_then(|x| x["version"].as_str())
                        .map(|x| x.to_string())
                        .ok_or("Failed to parse version")?,
                    r#type: PackageType::Npm,
                };
                Ok(pkg)
            })
            .collect::<Result<Vec<_>, _>>()
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
        let (_, entries) = yarn::parse(&self.0).map_err(|_e| "Failed to parse yarn lock file")?;
        Ok(entries)
    }
}

impl Parseable for GemLock {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(GemLock(std::fs::read_to_string(filename)?))
    }

    /// Parses `Gemfile.lock` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let (_, entries) = gem::parse(&self.0).map_err(|_e| "Failed to parse gem lock file")?;
        Ok(entries)
    }
}

impl Parseable for PyRequirements {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(PyRequirements(std::fs::read_to_string(filename)?))
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
        Ok(PipFile(std::fs::read_to_string(filename)?))
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
                            name: k.as_str().to_string(),
                            version: v.replace("==", "").trim().to_string(),
                            r#type: PackageType::Python,
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

mod tests {
    #[cfg(test)]
    use super::*;

    #[test]
    fn lock_parse_package() {
        let parser = PackageLock::new(Path::new("tests/fixtures/package-lock.json")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 17);
        assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
        assert_eq!(pkgs[0].version, "1.1.0");
        assert_eq!(pkgs[0].r#type, PackageType::Npm);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "yargs-parser");
        assert_eq!(last.version, "20.2.4");
        assert_eq!(last.r#type, PackageType::Npm);
    }

    #[test]
    fn lock_parse_yarn() {
        for p in &[
            "tests/fixtures/yarn.lock",
            "tests/fixtures/yarn.trailing_newlines.lock",
        ] {
            let parser = YarnLock::new(Path::new(p)).unwrap();

            let pkgs = parser.parse().unwrap();
            assert_eq!(pkgs.len(), 17);
            assert_eq!(pkgs[0].name, "@yarnpkg/lockfile");
            assert_eq!(pkgs[0].version, "1.1.0");
            assert_eq!(pkgs[0].r#type, PackageType::Npm);

            let last = pkgs.last().unwrap();
            assert_eq!(last.name, "yargs");
            assert_eq!(last.version, "16.2.0");
            assert_eq!(last.r#type, PackageType::Npm);
        }
    }

    #[should_panic]
    #[test]
    fn lock_parse_yarn_malformed_fails() {
        let parser = YarnLock::new(Path::new("tests/fixtures/yarn.lock.bad")).unwrap();

        parser.parse().unwrap();
    }

    #[test]
    fn lock_parse_gem() {
        let parser = GemLock::new(Path::new("tests/fixtures/Gemfile.lock")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 214);
        assert_eq!(pkgs[0].name, "CFPropertyList");
        assert_eq!(pkgs[0].version, "2.3.6");
        assert_eq!(pkgs[0].r#type, PackageType::Ruby);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "xpath");
        assert_eq!(last.version, "3.2.0");
        assert_eq!(last.r#type, PackageType::Ruby);
    }

    #[test]
    fn parse_requirements() {
        let parser = PyRequirements::new(Path::new("tests/fixtures/requirements.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 129);
        assert_eq!(pkgs[0].name, "PyYAML");
        assert_eq!(pkgs[0].version, "5.4.1");
        assert_eq!(pkgs[0].r#type, PackageType::Python);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "livy");
        assert_eq!(last.version, "0.7.3");
        assert_eq!(last.r#type, PackageType::Python);
    }

    #[test]
    fn parse_requirements_complex() {
        let parser =
            PyRequirements::new(Path::new("tests/fixtures/complex-requirements.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 5);
        assert_eq!(pkgs[0].name, "docopt");
        assert_eq!(pkgs[0].version, "0.6.1");
        assert_eq!(pkgs[0].r#type, PackageType::Python);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "FooProject5");
        assert_eq!(last.version, "1.5");
        assert_eq!(last.r#type, PackageType::Python);
    }

    #[test]
    fn parse_pipfile() {
        let parser = PipFile::new(Path::new("tests/fixtures/Pipfile")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 4);

        for pkg in &pkgs {
            if pkg.name == "pypresence" {
                assert_eq!(pkg.version, "4.0.0");
                assert_eq!(pkg.r#type, PackageType::Python);
            } else if pkg.name == "chromedriver-py" {
                assert_eq!(pkg.version, "91.0.4472.19");
                assert_eq!(pkg.r#type, PackageType::Python);
            } else if pkg.name == "requests" {
                assert_eq!(pkg.version, "2.24.0");
                assert_eq!(pkg.r#type, PackageType::Python);
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
                assert_eq!(pkg.r#type, PackageType::Python);
            } else if pkg.name == "certifi" {
                assert_eq!(pkg.version, "2017.7.27.1");
                assert_eq!(pkg.r#type, PackageType::Python);
            } else if pkg.name == "unittest2" {
                assert_eq!(pkg.version, "1.1.0");
                assert_eq!(pkg.r#type, PackageType::Python);
            }
        }
    }
}
