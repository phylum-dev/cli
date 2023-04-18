use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::PackageType;
use serde::Deserialize;

use super::parsers::pypi;
use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

pub struct PyRequirements;
pub struct PipFile;
pub struct Poetry;

impl Parse for PyRequirements {
    /// Parses `requirements.txt` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let (_, entries) = pypi::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse requirements file")?;
        Ok(entries)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("requirements.txt"))
    }

    fn is_path_manifest(&self, _path: &Path) -> bool {
        false
    }
}

impl Parse for PipFile {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let mut piplock: PipLock = serde_json::from_str(data)?;

        // Combine normal and dev dependencies.
        piplock.default.extend(piplock.develop);

        piplock
            .default
            .drain()
            .map(|(name, package)| {
                let version = if let Some(git) = package.git {
                    let git_ref = package
                        .git_ref
                        .ok_or_else(|| anyhow!("Git dependency {name:?} is missing git ref"))?;
                    PackageVersion::Git(format!("{git}#{git_ref}"))
                } else if let Some(path) = package.path {
                    PackageVersion::Path(Some(path.into()))
                } else if let Some(url) = package.file {
                    PackageVersion::DownloadUrl(url)
                } else {
                    let version = package.version.ok_or_else(|| {
                        anyhow!("Registry dependency {name:?} is missing version")
                    })?;
                    match version.strip_prefix("==") {
                        Some(version) => PackageVersion::FirstParty(version.into()),
                        None => {
                            return Err(anyhow!(
                                "Invalid lockfile version {version:?} for package {name:?}"
                            ))
                        },
                    }
                };

                Ok(Package { name, version, package_type: PackageType::PyPi })
            })
            .collect()
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Pipfile.lock"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Pipfile"))
    }
}

#[derive(Deserialize, Debug)]
struct PipLock {
    #[serde(default)]
    default: HashMap<String, PipPackage>,
    #[serde(default)]
    develop: HashMap<String, PipPackage>,
}

#[derive(Deserialize, Debug)]
struct PipPackage {
    version: Option<String>,
    git: Option<String>,
    #[serde(rename = "ref")]
    git_ref: Option<String>, // TODO: Test that this is also the name for the hash.
    path: Option<String>,
    file: Option<String>,
}

impl Parse for Poetry {
    /// Parses `poetry.lock` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let mut lock: PoetryLock = toml::from_str(data)?;

        // Warn if the version of this lockfile might not be supported.
        if !lock.metadata.lock_version.starts_with("1.")
            && !lock.metadata.lock_version.starts_with("2.")
        {
            log::debug!(
                "Expected poetry lockfile version ^1.0.0 or ^2.0.0, found {}.",
                lock.metadata.lock_version
            );
        }

        lock.packages.drain(..).map(Package::try_from).collect()
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("poetry.lock"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("pyproject.toml"))
    }
}

#[derive(Deserialize, Debug)]
struct PoetryLock {
    #[serde(rename = "package")]
    packages: Vec<PoetryPackage>,
    metadata: PoetryMetadata,
}

#[derive(Deserialize, Debug)]
struct PoetryPackage {
    name: String,
    version: String,
    source: Option<PackageSource>,
}

impl TryFrom<PoetryPackage> for Package {
    type Error = anyhow::Error;

    fn try_from(package: PoetryPackage) -> anyhow::Result<Self> {
        let source = match package.source {
            Some(source) => source,
            None => {
                return Ok(Self {
                    name: package.name,
                    version: PackageVersion::FirstParty(package.version),
                    package_type: PackageType::PyPi,
                });
            },
        };

        let version = match source.source_type.as_str() {
            "legacy" => {
                if source.url == "https://pypi.org/simple" {
                    PackageVersion::FirstParty(package.version)
                } else {
                    PackageVersion::ThirdParty(ThirdPartyVersion {
                        registry: source.url,
                        version: package.version,
                    })
                }
            },
            "directory" | "file" => PackageVersion::Path(Some(source.url.into())),
            "git" => {
                let reference = source
                    .resolved_reference
                    .ok_or_else(|| anyhow!("Git dependency missing resolved_reference field"))?;
                PackageVersion::Git(format!("{}#{}", source.url, reference))
            },
            "url" => PackageVersion::DownloadUrl(source.url),
            source_type => return Err(anyhow!("Unknown package source_type: {source_type:?}")),
        };

        Ok(Self { name: package.name, version, package_type: PackageType::PyPi })
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
        let pkgs = PyRequirements
            .parse(include_str!("../../tests/fixtures/requirements-locked.txt"))
            .unwrap();
        assert_eq!(pkgs.len(), 12);

        let expected_pkgs = [
            Package {
                name: "alembic".into(),
                version: PackageVersion::FirstParty("1.10.3".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "amqp".into(),
                version: PackageVersion::FirstParty("5.0.9".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "attrs".into(),
                version: PackageVersion::FirstParty("20.2.0".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "flask".into(),
                version: PackageVersion::FirstParty("2.2.2".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "requests".into(),
                version: PackageVersion::FirstParty("2.28.1".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "werkzeug".into(),
                version: PackageVersion::FirstParty("2.9.2".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "attr".into(),
                version: PackageVersion::Path(Some("file:///tmp/attr".into())),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "numpy".into(),
                version: PackageVersion::Path(Some("file:///tmp/testing/numpy-1.23.5-pp38-pypy38_pp73-win_amd64.whl".into())),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "git-for-pip-example".into(),
                version: PackageVersion::Git("git+https://github.com/matiascodesal/git-for-pip-example.git@v1.0.0".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "tomli".into(),
                version: PackageVersion::DownloadUrl("https://files.pythonhosted.org/packages/97/75/10a9ebee3fd790d20926a90a2547f0bf78f371b2f13aa822c759680ca7b9/tomli-2.0.1-py3-none-any.whl".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "phylum".into(),
                version: PackageVersion::Git("git+ssh://git@github.com/phylum-dev/phylum-ci.git#7d6d859ad368d1ab0a933f24679e3d3c08a40eac".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "editable".into(),
                version: PackageVersion::Path(Some("/tmp/editable".into())),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg), "missing package: {expected_pkg:?}");
        }
    }

    #[test]
    fn fails_loose_requirements() {
        // Ensure the entire file isn't valid.
        let unlocked = include_str!("../../tests/fixtures/requirements-unlocked.txt");
        let result = PyRequirements.parse(unlocked);
        assert!(result.is_err());

        // Ensure no individual line is valid.
        for line in unlocked.lines() {
            let result = PyRequirements.parse(line);
            assert!(result.is_err(), "Invalid valid dependency: {result:?}");
        }
    }

    #[test]
    fn parse_pipfile() {
        let result = PipFile.parse(include_str!("../../tests/fixtures/Pipfile"));
        assert!(result.is_err());
    }

    #[test]
    fn lock_parse_pipfile() {
        let pkgs = PipFile.parse(include_str!("../../tests/fixtures/Pipfile.lock")).unwrap();
        assert_eq!(pkgs.len(), 30);

        let expected_pkgs = [
            Package {
                name: "jdcal".into(),
                version: PackageVersion::FirstParty("1.3".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "certifi".into(),
                version: PackageVersion::FirstParty("2017.7.27.1".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "unittest2".into(),
                version: PackageVersion::FirstParty("1.1.0".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "django".into(),
                version: PackageVersion::Git("https://github.com/django/django.git#1.11.4".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "e1839a8".into(),
                version: PackageVersion::Path(Some(".".into())),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "e682b37".into(),
                version: PackageVersion::DownloadUrl(
                    "https://github.com/divio/django-cms/archive/release/3.4.x.zip".into(),
                ),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_poetry_lock_v1() {
        let pkgs = Poetry.parse(include_str!("../../tests/fixtures/poetry.lock")).unwrap();
        assert_eq!(pkgs.len(), 48);

        let expected_pkgs = [
            Package {
                name: "cachecontrol".into(),
                version: PackageVersion::FirstParty("0.12.10".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "flask".into(),
                version: PackageVersion::FirstParty("2.1.1".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "poetry".into(),
                version: PackageVersion::Git("https://github.com/python-poetry/poetry.git#4bc181b06ff9780791bc9e3d5b11bb807ca29d70".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "autopep8".into(),
                version: PackageVersion::ThirdParty(ThirdPartyVersion {
                    registry: "https://example.com/api/pypi/python/simple".into(),
                    version: "1.5.6".into(),
                }),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "directory-test".into(),
                version: PackageVersion::Path(Some("directory_test".into())),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "requests".into(),
                version: PackageVersion::Path(Some("requests/requests-2.27.1-py2.py3-none-any.whl".into())),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "toml".into(),
                version: PackageVersion::DownloadUrl("https://files.pythonhosted.org/packages/be/ba/1f744cdc819428fc6b5084ec34d9b30660f6f9daaf70eead706e3203ec3c/toml-0.10.2.tar.gz".into()),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_poetry_lock_v2() {
        let pkgs = Poetry.parse(include_str!("../../tests/fixtures/poetry_v2.lock")).unwrap();
        assert_eq!(pkgs.len(), 9);

        let expected_pkgs = [
            Package {
                name: "certifi".into(),
                version: PackageVersion::FirstParty("2020.12.5".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "pywin32".into(),
                version: PackageVersion::FirstParty("227".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "docker".into(),
                version: PackageVersion::FirstParty("4.3.1".into()),
                package_type: PackageType::PyPi,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg), "missing package {expected_pkg:?}");
        }
    }
}
