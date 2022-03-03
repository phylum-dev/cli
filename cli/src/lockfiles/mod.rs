use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::marker::Sized;
use std::path::Path;

use serde_json::Value;

use phylum_types::ecosystems::maven::{Dependency, Plugin, Project};
use phylum_types::types::package::{PackageDescriptor, PackageType};

mod parsers;
use parsers::{gem, gradle_dep, pypi, yarn};

pub struct PackageLock(String);
pub struct YarnLock(String);
pub struct GemLock(String);
pub struct PyRequirements(String);
pub struct PipFile(String);
pub struct Pom(String);
pub struct GradleDeps(String);

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
                    package_type: PackageType::Npm,
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
                            name: k.as_str().to_string().to_lowercase(),
                            version: v.replace("==", "").trim().to_string(),
                            package_type: PackageType::Python,
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

impl Parseable for GradleDeps {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(GradleDeps(std::fs::read_to_string(filename)?))
    }

    /// Parses `requirements.txt` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let (_, entries) =
            gradle_dep::parse(&self.0).map_err(|_e| "Failed to parse requirements file")?;
        Ok(entries)
    }
}

impl Parseable for Pom {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(Pom(std::fs::read_to_string(filename)?))
    }

    /// Parses maven effecti-pom files into a vec of packages
    fn parse(&self) -> ParseResult {
        // Get plugin dependencies
        fn get_plugin_deps(plugins: &[Plugin]) -> Vec<Dependency> {
            plugins
                .iter()
                .flat_map(|plugin| {
                    let mut deps = plugin.dependencies.clone().unwrap_or_default().dependencies;
                    deps.push(Dependency {
                        group_id: plugin.group_id.clone(),
                        artifact_id: plugin.artifact_id.clone(),
                        version: plugin.version.clone(),
                        dtype: None,
                        classifier: None,
                        scope: None,
                        system_path: None,
                        exclusions: None,
                        optional: None,
                    });
                    deps
                })
                .collect::<Vec<_>>()
        }

        // Get project reference
        let pom: Project = serde_xml_rs::from_str(&self.0).unwrap_or_default();

        // Get project dependencies
        let mut dependencies = pom.dependencies.unwrap_or_default().dependencies;

        // Get the reporting dependencies
        let mut reporting_dependencies = get_plugin_deps(
            &pom.reporting
                .unwrap_or_default()
                .plugins
                .unwrap_or_default(),
        );

        // Combine plugins and plugin dependencies
        let mut build_plugins = get_plugin_deps(
            &pom.build
                .as_ref()
                .and_then(|b| b.plugins.clone())
                .unwrap_or_default(),
        );

        // Get build artifacts
        let build_ext = &pom
            .build
            .unwrap_or_default()
            .extensions
            .unwrap_or_default()
            .iter()
            .map(|ext| Dependency {
                group_id: ext.group_id.clone(),
                artifact_id: ext.artifact_id.clone(),
                version: ext.version.clone(),
                dtype: None,
                classifier: None,
                scope: None,
                system_path: None,
                exclusions: None,
                optional: None,
            })
            .collect::<Vec<_>>();

        let mut profile_dependencies = pom
            .profiles
            .unwrap_or_default()
            .profiles
            .into_iter()
            .flat_map(|p| {
                let mut p_deps = p.dependencies.unwrap_or_default().dependencies;
                let p_report_plugins =
                    get_plugin_deps(&p.reporting.unwrap_or_default().plugins.unwrap_or_default());
                p_deps.extend(p_report_plugins);
                p_deps
            })
            .collect::<Vec<_>>();

        dependencies.append(&mut reporting_dependencies);
        dependencies.append(&mut build_plugins);
        dependencies.extend(build_ext.to_owned());
        dependencies.append(&mut profile_dependencies);
        dependencies.dedup();
        dependencies
            .iter()
            .filter_map(|dep| {
                dep.version.as_ref().map(|s| {
                    Ok(PackageDescriptor {
                        name: format!(
                            "{}:{}",
                            &dep.group_id.clone().unwrap_or_default(),
                            &dep.artifact_id.clone().unwrap_or_default()
                        ),
                        version: s.into(),
                        package_type: PackageType::Maven,
                    })
                })
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
        assert_eq!(pkgs[0].package_type, PackageType::Npm);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "yargs-parser");
        assert_eq!(last.version, "20.2.4");
        assert_eq!(last.package_type, PackageType::Npm);
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
            assert_eq!(pkgs[0].package_type, PackageType::Npm);

            let last = pkgs.last().unwrap();
            assert_eq!(last.name, "yargs");
            assert_eq!(last.version, "16.2.0");
            assert_eq!(last.package_type, PackageType::Npm);
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
        assert_eq!(pkgs[0].package_type, PackageType::Ruby);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "xpath");
        assert_eq!(last.version, "3.2.0");
        assert_eq!(last.package_type, PackageType::Ruby);
    }

    #[test]
    fn parse_requirements() {
        let parser = PyRequirements::new(Path::new("tests/fixtures/requirements.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 130);
        assert_eq!(pkgs[0].name, "pyyaml");
        assert_eq!(pkgs[0].version, "5.4.1");
        assert_eq!(pkgs[0].package_type, PackageType::Python);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "zope.interface");
        assert_eq!(last.version, "5.4.0");
        assert_eq!(last.package_type, PackageType::Python);
    }

    #[test]
    fn parse_requirements_complex() {
        let parser =
            PyRequirements::new(Path::new("tests/fixtures/complex-requirements.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 5);
        assert_eq!(pkgs[0].name, "docopt");
        assert_eq!(pkgs[0].version, "0.6.1");
        assert_eq!(pkgs[0].package_type, PackageType::Python);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "fooproject5");
        assert_eq!(last.version, "1.5");
        assert_eq!(last.package_type, PackageType::Python);
    }

    #[test]
    fn parse_pipfile() {
        let parser = PipFile::new(Path::new("tests/fixtures/Pipfile")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 4);

        for pkg in &pkgs {
            if pkg.name == "pypresence" {
                assert_eq!(pkg.version, "4.0.0");
                assert_eq!(pkg.package_type, PackageType::Python);
            } else if pkg.name == "chromedriver-py" {
                assert_eq!(pkg.version, "91.0.4472.19");
                assert_eq!(pkg.package_type, PackageType::Python);
            } else if pkg.name == "requests" {
                assert_eq!(pkg.version, "2.24.0");
                assert_eq!(pkg.package_type, PackageType::Python);
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
                assert_eq!(pkg.package_type, PackageType::Python);
            } else if pkg.name == "certifi" {
                assert_eq!(pkg.version, "2017.7.27.1");
                assert_eq!(pkg.package_type, PackageType::Python);
            } else if pkg.name == "unittest2" {
                assert_eq!(pkg.version, "1.1.0");
                assert_eq!(pkg.package_type, PackageType::Python);
            }
        }
    }

    #[test]
    fn lock_parse_gradledeps() {
        let parser = GradleDeps::new(Path::new("tests/fixtures/gradle-dependencies.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 43);
        assert_eq!(pkgs[0].name, "com.google.guava:guava");
        assert_eq!(pkgs[0].version, "23.3-jre");
        assert_eq!(pkgs[0].package_type, PackageType::Maven);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "commons-codec:commons-codec");
        assert_eq!(last.version, "1.9");
        assert_eq!(last.package_type, PackageType::Maven);
    }

    #[test]
    fn lock_parse_effective_pom() {
        let parser = Pom::new(Path::new("tests/fixtures/effective-pom.xml")).unwrap();

        let mut pkgs = parser.parse().unwrap();
        pkgs.sort_by(|a, b| a.version.cmp(&b.version));
        assert_eq!(pkgs.len(), 16);
        assert_eq!(pkgs[0].name, "com.bitalino:bitalino-java-sdk");
        assert_eq!(pkgs[0].version, "1.1.0");
        assert_eq!(pkgs[0].package_type, PackageType::Maven);

        assert_eq!(pkgs[1].name, "org.codehaus.mojo:exec-maven-plugin");
        assert_eq!(pkgs[1].version, "1.2.1");
        assert_eq!(pkgs[1].package_type, PackageType::Maven);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "org.apache.maven.plugins:maven-site-plugin");
        assert_eq!(last.version, "3.3");
        assert_eq!(last.package_type, PackageType::Maven);
    }
}
