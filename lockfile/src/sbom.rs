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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct PkgInfo {
    pub pkg_name: String,
    pub pkg_type: PackageType,
}

impl TryFrom<&PackageInformation> for PkgInfo {
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
        let pkg_type =
            PackageType::from_str(purl.ty()).map_err(|_| anyhow!("Unrecognized ecosystem"))?;
        let pkg_name = match purl.namespace() {
            Some(ns) => format!("{}/{}", ns, purl.name()),
            None => purl.name().into(),
        };

        Ok(PkgInfo { pkg_name, pkg_type })
    }
}

pub struct Sbom;

impl Parse for Sbom {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let mut lock: Spdx = serde_json::from_str(data).unwrap_or(serde_yaml::from_str(data)?);

        let m = lock
            .packages
            .drain(..)
            .filter_map(|pkg| {
                let pkg_info = match PkgInfo::try_from(&pkg).ok() {
                    Some(pi) => pi,
                    None => PkgInfo { pkg_name: pkg.name, pkg_type: PackageType::Sbom },
                };

                match (pkg_info.pkg_type, pkg.version_info) {
                    (PackageType::Sbom, Some(version)) => Some(Package {
                        name: pkg_info.pkg_name,
                        version: PackageVersion::ThirdParty(ThirdPartyVersion {
                            version,
                            registry: pkg.download_location,
                        }),
                    }),
                    (_, Some(version)) => Some(Package {
                        name: pkg_info.pkg_name.clone(),
                        version: PackageVersion::FirstParty(version),
                    }),
                    _ => None,
                }
            })
            .collect::<Vec<Package>>();

        Ok(m)
    }

    fn package_type(&self) -> phylum_types::types::package::PackageType {
        PackageType::Sbom
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
        assert_eq!(pkgs.len(), 5);

        let expected_pkgs = [Package {
            name: "org.hamcrest/hamcrest-core".into(),
            version: PackageVersion::FirstParty(FirstPartyVersion {
                version: "1.3".into(),
                registry: PackageType::Maven,
            }),
        }];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_spdx_2_3_yaml() {
        let pkgs = Sbom.parse(include_str!("../../tests/fixtures/spdx-2.3.spdx.yaml")).unwrap();
        assert_eq!(pkgs.len(), 3);

        let expected_pkgs = [Package {
            name: "org.apache.jena/apache-jena".into(),
            version: PackageVersion::FirstParty(FirstPartyVersion {
                version: "3.12.0".into(),
                registry: PackageType::Maven,
            }),
        }];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
