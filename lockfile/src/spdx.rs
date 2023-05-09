use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use lazy_static::lazy_static;
use nom::error::convert_error;
use nom::Finish;
use packageurl::PackageUrl;
use phylum_types::types::package::PackageType;
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;
use urlencoding::decode;

use crate::parsers::spdx;
use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

#[derive(Error, Debug)]
#[error("Could not determine ecosystem")]
struct UnknownEcosystem;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SpdxInfo {
    packages: Vec<PackageInformation>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageInformation {
    pub(crate) name: String,
    pub(crate) version_info: Option<String>,
    pub(crate) download_location: String,
    pub(crate) external_refs: Vec<ExternalRefs>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalRefs {
    pub(crate) reference_category: ReferenceCategory,
    pub(crate) reference_locator: String,
    pub(crate) reference_type: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub(crate) enum ReferenceCategory {
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

fn type_from_url(url: &str) -> anyhow::Result<PackageType> {
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
        bail!("Unknown download URL: {:?}", url)
    }
}

fn from_purl(pkg_url: &str, pkg_info: &PackageInformation) -> anyhow::Result<Package> {
    let purl = PackageUrl::from_str(pkg_url)?;

    let package_type = PackageType::from_str(purl.ty())
        .or_else(|_| type_from_url(&pkg_info.download_location))
        .context(UnknownEcosystem)?;

    let name = match (package_type, purl.namespace()) {
        (PackageType::Maven, Some(ns)) => format!("{}:{}", ns, purl.name()),
        (PackageType::Npm | PackageType::Golang, Some(ns)) => format!("{}/{}", ns, purl.name()),
        _ => purl.name().into(),
    };

    let pkg_version = match (&pkg_info.version_info, purl.version()) {
        (Some(v), _) => v.to_string(),
        (None, Some(v)) => v.into(),
        _ => bail!("Version not found for `{}`", pkg_info.name),
    };

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

fn from_locator(registry: &str, locator: &str) -> anyhow::Result<Package> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(([^:]*:)*[^@]+)@([^:]+)$").unwrap();
    }

    let package_type = PackageType::from_str(registry).map_err(|_| UnknownEcosystem)?;

    let captures = RE.captures(locator).expect("Invalid locator");
    let name = decode(captures.get(1).expect("Invalid package name").as_str())?;
    let version = captures.get(3).expect("Invalid package version").as_str();

    Ok(Package {
        name: name.into(),
        version: PackageVersion::FirstParty(version.into()),
        package_type,
    })
}

impl TryFrom<&PackageInformation> for Package {
    type Error = anyhow::Error;

    fn try_from(pkg_info: &PackageInformation) -> anyhow::Result<Self> {
        pkg_info
            .external_refs
            .iter()
            .find_map(|external| match external.reference_category {
                ReferenceCategory::PackageManager => {
                    if external.reference_type == "purl" {
                        Some(from_purl(&external.reference_locator, pkg_info))
                    } else {
                        Some(from_locator(&external.reference_type, &external.reference_locator))
                    }
                },
                _ => None,
            })
            .ok_or(anyhow!("Missing package locator for {}", pkg_info.name))?
    }
}

pub struct Spdx;

impl Parse for Spdx {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let packages_info = if let Ok(lock) = serde_json::from_str::<SpdxInfo>(data) {
            lock.packages
        } else if let Ok(lock) = serde_yaml::from_str::<SpdxInfo>(data) {
            lock.packages
        } else {
            spdx::parse(data).finish().map_err(|e| anyhow!(convert_error(data, e)))?.1
        };

        let mut packages = Vec::new();
        for package_info in packages_info {
            match Package::try_from(&package_info) {
                Ok(pkg) => packages.push(pkg),
                Err(e) => {
                    if e.is::<UnknownEcosystem>() {
                        log::warn!("{:?}", e)
                    } else {
                        bail!(e)
                    }
                },
            }
        }

        Ok(packages)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.ends_with(".spdx.json")
            || path.ends_with(".spdx.yaml")
            || path.ends_with(".spdx.yml")
            || path.ends_with(".spdx")
    }

    fn is_path_manifest(&self, _path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_spdx_2_2_json() {
        let pkgs = Spdx.parse(include_str!("../../tests/fixtures/spdx-2.2.spdx.json")).unwrap();
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
        let pkgs = Spdx.parse(include_str!("../../tests/fixtures/spdx-2.3.spdx.yaml")).unwrap();
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
        let pkgs = Spdx.parse(include_str!("../../tests/fixtures/spdx-2.3.spdx.json")).unwrap();
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

        let error = Spdx.parse(&data).err().unwrap();
        assert!(error.to_string().contains("Missing package locator"))
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

        let error = Spdx.parse(&data).err().unwrap();
        assert!(error.to_string().contains("Version"))
    }

    #[test]
    fn unsupported_ecosystem() {
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
                },
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": "pkg:tbd/colors/colors"
                }]
            }]
        }).to_string();

        let pkgs = Spdx.parse(&data).unwrap();
        assert!(pkgs.is_empty())
    }

    #[test]
    fn tag_value_fail_missing_purl() {
        let data = r##"SPDXVersion: SPDX-2.3
            DataLicense: CC0-1.0
            DocumentNamespace: http://spdx.org/spdxdocs/spdx-example-444504E0-4F89-41D3-9A0C-0305E82C3301
            DocumentName: SPDX-Tools-v2.0
            SPDXID: SPDXRef-DOCUMENT
            DocumentComment: <text>This document was created using SPDX 2.0 using licenses from the web site.</text>

            ## Package Information
            PackageName: Jena
            SPDXID: SPDXRef-fromDoap-0
            PackageVersion: 3.12.0
            PackageDownloadLocation: https://search.maven.org/remotecontent?filepath=org/apache/jena/apache-jena/3.12.0/apache-jena-3.12.0.tar.gz
            PackageHomePage: http://www.openjena.org/
            FilesAnalyzed: false

            ## Package Information
            PackageName: @colors/colors
            SPDXID: SPDXRef-Package-npm--colors-colors-2f307524f9ea3c7b
            PackageVersion: 1.5.0
            PackageDownloadLocation: http://github.com/DABH/colors.js.git"##;

        let error = Spdx.parse(data).err().unwrap();
        assert!(error.to_string().contains("Missing package locator"))
    }

    #[test]
    fn tag_value_fail_missing_version() {
        let data = r##"SPDXVersion: SPDX-2.3
            DataLicense: CC0-1.0
            DocumentNamespace: http://spdx.org/spdxdocs/spdx-example-444504E0-4F89-41D3-9A0C-0305E82C3301
            DocumentName: SPDX-Tools-v2.0
            SPDXID: SPDXRef-DOCUMENT
            DocumentComment: <text>This document was created using SPDX 2.0 using licenses from the web site.</text>

            ## Package Information
            PackageName: Jena
            SPDXID: SPDXRef-fromDoap-0
            PackageDownloadLocation: https://search.maven.org/remotecontent?filepath=org/apache/jena/apache-jena/3.12.0/apache-jena-3.12.0.tar.gz
            PackageHomePage: http://www.openjena.org/
            ExternalRef: PACKAGE-MANAGER purl pkg:maven/org.apache.jena/apache-jena
            FilesAnalyzed: false

            ## Package Information
            PackageName: @colors/colors
            SPDXID: SPDXRef-Package-npm--colors-colors-2f307524f9ea3c7b
            PackageVersion: 1.5.0
            PackageDownloadLocation: http://github.com/DABH/colors.js.git"##;

        let error = Spdx.parse(data).err().unwrap();
        assert!(error.to_string().contains("Version"))
    }

    #[test]
    fn tag_value_unsupported_ecosystem() {
        let data = r##"SPDXVersion: SPDX-2.3
            DataLicense: CC0-1.0
            DocumentNamespace: http://spdx.org/spdxdocs/spdx-example-444504E0-4F89-41D3-9A0C-0305E82C3301
            DocumentName: SPDX-Tools-v2.0
            SPDXID: SPDXRef-DOCUMENT
            DocumentComment: <text>This document was created using SPDX 2.0 using licenses from the web site.</text>

            ## Package Information
            PackageName: TBD
            SPDXID: SPDXRef-fromDoap-0
            PackageDownloadLocation: https://search.maven.org/remotecontent?filepath=org/apache/jena/apache-jena/3.12.0/apache-jena-3.12.0.tar.gz
            PackageHomePage: http://www.openjena.org/
            ExternalRef: PACKAGE-MANAGER purl pkg:tbd/org.apache.jena/apache-jena
            FilesAnalyzed: false

            ## Package Information
            PackageName: @colors/colors
            SPDXID: SPDXRef-Package-npm--colors-colors-2f307524f9ea3c7b
            PackageVersion: 1.5.0
            PackageDownloadLocation: http://github.com/DABH/colors.js.git"##;

        let pkgs = Spdx.parse(data).unwrap();
        assert!(pkgs.is_empty())
    }

    #[test]
    fn parse_spdx_2_2_tag_value() {
        let pkgs = Spdx.parse(include_str!("../../tests/fixtures/spdx-2.2.spdx")).unwrap();
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
    fn pkg_locator() {
        let pkgs = Spdx.parse(include_str!("../../tests/fixtures/locator.spdx.json")).unwrap();

        let expected_pkgs = [
            Package {
                name: "@npmcli/fs".into(),
                version: PackageVersion::FirstParty("2.1.2".into()),
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
                name: "org.jruby:jruby-complete".into(),
                version: PackageVersion::FirstParty("9.3.7.0".into()),
                package_type: PackageType::Maven,
            },
            Package {
                name: "Newtonsoft.Json".into(),
                version: PackageVersion::FirstParty("13.0.1".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "gopkg.in/yaml.v2".into(),
                version: PackageVersion::FirstParty("v2.3.0".into()),
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
    fn pkg_locator_tag_value() {
        let pkgs = Spdx.parse(include_str!("../../tests/fixtures/locator.spdx")).unwrap();

        let expected_pkgs = [
            Package {
                name: "org.jruby:jruby-complete".into(),
                version: PackageVersion::FirstParty("9.3.7.0".into()),
                package_type: PackageType::Maven,
            },
            Package {
                name: "org.jruby:jruby-complete".into(),
                version: PackageVersion::FirstParty("9.2.1.0".into()),
                package_type: PackageType::Maven,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }
}
