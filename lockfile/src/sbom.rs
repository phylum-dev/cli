use std::str::FromStr;

use anyhow::{anyhow, Context};
use packageurl::PackageUrl;
use phylum_types::types::package::PackageType;
use serde::{Deserialize, Serialize};

use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Spdx {
    pub spdx_version: String,
    pub packages: Vec<PackageInformation>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackageInformation {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "versionInfo", skip_serializing_if = "Option::is_none", default)]
    pub version_info: Option<String>,
    #[serde(rename = "downloadLocation")]
    pub download_location: String,
    #[serde(rename = "externalRefs", skip_serializing_if = "Vec::is_empty", default)]
    pub external_refs: Vec<ExternalRefs>,
}

impl Default for PackageInformation {
    fn default() -> Self {
        Self {
            name: "NOASSERTION".to_string(),
            version_info: None,
            download_location: "NOASSERTION".to_string(),
            external_refs: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRefs {
    pub reference_category: ReferenceCategory,
    pub reference_locator: String,
    pub reference_type: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Clone)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum ReferenceCategory {
    Other,
    // older schema uses _
    #[serde(alias = "PERSISTENT_ID")]
    PersistentId,
    Security,
    // older schema uses _
    #[serde(alias = "PACKAGE_MANAGER")]
    PackageManager,
}

fn type_from_url(url: &str) -> Result<PackageType, ()> {
    if url.starts_with("https://registry.npmjs.org")
        | url.starts_with("https://registry.yarnpkg.com")
    {
        Ok(PackageType::Npm)
    } else if url.starts_with("https://rubygems.org") {
        Ok(PackageType::RubyGems)
    } else if url.starts_with("https://files.pythonhosted.org") {
        Ok(PackageType::PyPi)
    } else if url.starts_with("https://repo1.maven.org") {
        Ok(PackageType::Maven)
    } else if url.starts_with("https://api.nuget.org") {
        Ok(PackageType::Nuget)
    } else if url.starts_with("https://proxy.golang.org") {
        Ok(PackageType::Golang)
    } else if url.starts_with("https://crates.io") {
        Ok(PackageType::Cargo)
    } else {
        Err(())
    }
}

impl TryFrom<&PackageInformation> for Package {
    type Error = anyhow::Error;

    fn try_from(pkg_info: &PackageInformation) -> anyhow::Result<Self> {
        let pkg_url = pkg_info
            .external_refs
            .iter()
            .find_map(|external| match external.reference_category {
                ReferenceCategory::PackageManager => {
                    if external.reference_type == "purl" {
                        Some(&external.reference_locator)
                    } else {
                        None
                    }
                },
                _ => None,
            })
            .context("Package manager not found")?;

        let purl = PackageUrl::from_str(pkg_url).context("Unable to parse package url")?;
        let package_type = PackageType::from_str(purl.ty())
            .or_else(|_| type_from_url(&pkg_info.download_location))
            .map_err(|_| anyhow!("Unrecognized ecosystem"))?;
        let name = match purl.namespace() {
            Some(ns) => format!("{}/{}", ns, purl.name()),
            None => purl.name().into(),
        };
        let pkg_version = match (&pkg_info.version_info, purl.version()) {
            (Some(v), _) => Some(v.to_string()),
            (None, Some(v)) => Some(v.into()),
            _ => None,
        }
        .context("Unable to determine version")?;

        let version = purl
            .qualifiers()
            .iter()
            .find_map(|(key, value)| match key.as_ref() {
                "repository_url" => Some(PackageVersion::ThirdParty(ThirdPartyVersion {
                    version: pkg_version.clone(),
                    registry: value.to_string(),
                })),
                "download_url" => Some(PackageVersion::DownloadUrl(value.to_string())),
                "vcs_url" => {
                    if value.as_ref().starts_with("git+") {
                        Some(PackageVersion::Git(value.to_string()))
                    } else {
                        None
                    }
                },
                _ => None,
            })
            .unwrap_or(PackageVersion::FirstParty(pkg_version));

        Ok(Package { name, version, package_type })
    }
}

pub struct Sbom;

impl Parse for Sbom {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let mut lock: Spdx = serde_json::from_str(data).or_else(|_| serde_yaml::from_str(data))?;

        let packages = lock
            .packages
            .drain(..)
            .filter_map(|package_info| Package::try_from(&package_info).ok())
            .collect::<Vec<Package>>();

        Ok(packages)
    }

    fn is_path_lockfile(&self, path: &std::path::Path) -> bool {
        path.ends_with(".spdx.json") || path.ends_with(".spdx.yaml") || path.ends_with(".spdx.yml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spdx_2_2_json() {
        let pkgs = Sbom.parse(include_str!("../../tests/fixtures/spdx-2.2.spdx.json")).unwrap();
        assert_eq!(pkgs.len(), 4);

        let expected_pkgs = [Package {
            name: "org.hamcrest/hamcrest-core".into(),
            version: PackageVersion::FirstParty("1.3".into()),
            package_type: PackageType::Maven,
        }];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_spdx_2_3_yaml() {
        let pkgs = Sbom.parse(include_str!("../../tests/fixtures/spdx-2.3.spdx.yaml")).unwrap();
        assert_eq!(pkgs.len(), 1);

        let expected_pkgs = [Package {
            name: "org.apache.jena/apache-jena".into(),
            version: PackageVersion::FirstParty("3.12.0".into()),
            package_type: PackageType::Maven,
        }];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_spdx_2_3_json() {
        let pkgs = Sbom.parse(include_str!("../../tests/fixtures/spdx-2.3.spdx.json")).unwrap();
        assert_eq!(pkgs.len(), 2673);

        let expected_pkgs = [
            Package {
                name: "@colors/colors".into(),
                version: PackageVersion::FirstParty("1.5.0".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "CFPropertyList".into(),
                version: PackageVersion::FirstParty("2.3.6".into()),
                package_type: PackageType::RubyGems,
            },
            Package {
                name: "async-timeout".into(),
                version: PackageVersion::FirstParty("4.0.2".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "org.codehaus.classworlds/classworlds".into(),
                version: PackageVersion::FirstParty("1.1".into()),
                package_type: PackageType::Maven,
            },
            Package {
                name: "Newtonsoft.Json".into(),
                version: PackageVersion::FirstParty("13.0.1".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "dmitri.shuralyov.com/gpu/mtl".into(),
                version: PackageVersion::FirstParty("v0.0.0-20190408044501-666a987793e9".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "env_logger".into(),
                version: PackageVersion::FirstParty("0.8.4".into()),
                package_type: PackageType::Cargo,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
