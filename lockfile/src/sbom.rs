use std::str::FromStr;

use anyhow::anyhow;
use packageurl::PackageUrl;
use phylum_types::types::package::PackageType;
use serde::{Deserialize, Serialize};

use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Spdx {
    packages: Vec<PackageInformation>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PackageInformation {
    name: String,
    version_info: Option<String>,
    download_location: String,
    external_refs: Vec<ExternalRefs>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum SbomError {
    UnknownEcosytem(String),
    Purl,
    Version,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ExternalRefs {
    reference_category: ReferenceCategory,
    reference_locator: String,
    reference_type: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
enum ReferenceCategory {
    Other,
    // older schema uses _
    #[serde(alias = "PERSISTENT_ID")]
    PersistentId,
    Security,
    // older schema uses _
    #[serde(alias = "PACKAGE_MANAGER")]
    PackageManager,
    #[serde(other)]
    Unknown,
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
        Err(anyhow!("Unknown download URL: {:?}", url))
    }
}

impl TryFrom<&PackageInformation> for Package {
    type Error = SbomError;

    fn try_from(pkg_info: &PackageInformation) -> Result<Self, SbomError> {
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
            .ok_or(SbomError::Purl)?;

        let purl = PackageUrl::from_str(pkg_url).map_err(|_| SbomError::Purl)?;
        let purl_ty = purl.ty();
        let package_type = PackageType::from_str(purl.ty())
            .or_else(|_| type_from_url(&pkg_info.download_location))
            .map_err(|_| SbomError::UnknownEcosytem(purl_ty.into()))?;
        let name = match (package_type, purl.namespace()) {
            (PackageType::Maven, Some(ns)) => format!("{}:{}", ns, purl.name()),
            (_, Some(ns)) => format!("{}/{}", ns, purl.name()),
            _ => purl.name().into(),
        };

        let pkg_version = match (&pkg_info.version_info, purl.version()) {
            (Some(v), _) => Some(v.to_string()),
            (None, Some(v)) => Some(v.into()),
            _ => None,
        }
        .ok_or(SbomError::Version)?;

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
        let lock: Spdx = serde_json::from_str(data).or_else(|_| serde_yaml::from_str(data))?;

        let mut packages = Vec::new();
        for package_info in lock.packages {
            match Package::try_from(&package_info) {
                Ok(pkg) => packages.push(pkg),
                Err(e) => {
                    let pkg_name = &package_info.name;
                    let pkg_ver = package_info.version_info.unwrap_or_default();
                    match e {
                        SbomError::UnknownEcosytem(ecosystem) => {
                            log::warn!(
                                "{pkg_name}:{pkg_ver} uses an unsupported ecosystem: {ecosystem}",
                            )
                        },
                        SbomError::Purl => {
                            return Err(anyhow!("{pkg_name}:{pkg_ver} is missing a package url"))
                        },
                        SbomError::Version => {
                            return Err(anyhow!("{pkg_name} is missing version information"))
                        },
                    }
                },
            }
        }

        Ok(packages)
    }

    fn is_path_lockfile(&self, path: &std::path::Path) -> bool {
        path.ends_with(".spdx.json") || path.ends_with(".spdx.yaml") || path.ends_with(".spdx.yml")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_spdx_2_2_json() {
        let pkgs = Sbom.parse(include_str!("../../tests/fixtures/spdx-2.2.spdx.json")).unwrap();
        assert_eq!(pkgs.len(), 4);

        let expected_pkgs = [Package {
            name: "org.hamcrest:hamcrest-core".into(),
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
            name: "org.apache.jena:apache-jena".into(),
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
                name: "org.codehaus.classworlds:classworlds".into(),
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

    #[test]
    fn fail_missing_purl() {
        let data = json!({
              "spdxVersion": "SPDX-2.3",
              "dataLicense": "CC0-1.0",
              "SPDXID": "SPDXRef-DOCUMENT",
              "name": "sbom-example",
              "packages": [ {
                "name": "@colors/colors",
                "SPDXID": "SPDXRef-Package-npm--colors-colors-2f307524f9ea3c7b",
                "versionInfo": "1.5.0",
                "originator": "Person: DABH",
                "downloadLocation": "http://github.com/DABH/colors.js.git",
                "homepage": "https://github.com/DABH/colors.js",
                "sourceInfo": "acquired package info from installed node module manifest file: /usr/local/lib/node_modules/npm/node_modules/@colors/colors/package.json",
                "licenseConcluded": "MIT",
                "licenseDeclared": "MIT",
                "copyrightText": "NOASSERTION",
                "externalRefs": [
                {
                    "referenceCategory": "SECURITY",
                    "referenceType": "cpe23Type",
                    "referenceLocator": "cpe:2.3:a:\\@colors\\/colors:\\@colors\\/colors:1.5.0:*:*:*:*:*:*:*"
                },
                {
                    "referenceCategory": "SECURITY",
                    "referenceType": "cpe23Type",
                    "referenceLocator": "cpe:2.3:a:*:\\@colors\\/colors:1.5.0:*:*:*:*:*:*:*"
                }]
            }]
        }).to_string();

        let error = Sbom.parse(&data).err().unwrap();
        assert!(error.to_string().contains("missing a package url"))
    }

    #[test]
    fn fail_missing_version() {
        let data = json!({
              "spdxVersion": "SPDX-2.3",
              "dataLicense": "CC0-1.0",
              "SPDXID": "SPDXRef-DOCUMENT",
              "name": "sbom-example",
              "packages": [ {
                "name": "@colors/colors",
                "SPDXID": "SPDXRef-Package-npm--colors-colors-2f307524f9ea3c7b",
                "originator": "Person: DABH",
                "downloadLocation": "http://github.com/DABH/colors.js.git",
                "homepage": "https://github.com/DABH/colors.js",
                "sourceInfo": "acquired package info from installed node module manifest file: /usr/local/lib/node_modules/npm/node_modules/@colors/colors/package.json",
                "licenseConcluded": "MIT",
                "licenseDeclared": "MIT",
                "copyrightText": "NOASSERTION",
                "externalRefs": [
                {
                    "referenceCategory": "SECURITY",
                    "referenceType": "cpe23Type",
                    "referenceLocator": "cpe:2.3:a:\\@colors\\/colors:\\@colors\\/colors:1.5.0:*:*:*:*:*:*:*"
                },
                {
                    "referenceCategory": "SECURITY",
                    "referenceType": "cpe23Type",
                    "referenceLocator": "cpe:2.3:a:*:\\@colors\\/colors:1.5.0:*:*:*:*:*:*:*"
                },
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": "pkg:npm/%40colors/colors"
                }]
            }]
        }).to_string();

        let error = Sbom.parse(&data).err().unwrap();
        assert!(error.to_string().contains("missing version information"))
    }
}
