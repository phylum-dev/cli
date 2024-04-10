use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::PackageType;
use purl::GenericPurl;
use serde::Deserialize;
use urlencoding::decode;

use crate::parsers::spdx;
use crate::{
    determine_package_version, formatted_package_name, Package, PackageVersion, Parse,
    UnknownEcosystem,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpdxInfo {
    #[serde(rename = "SPDXID")]
    pub(crate) spdx_id: String,
    // Deprecated in v2.3 but kept for v2.2 compatability.
    #[serde(default)]
    pub(crate) document_describes: Vec<String>,
    pub(crate) packages: Vec<PackageInformation>,
    #[serde(default)]
    pub(crate) relationships: Vec<Relationship>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageInformation {
    pub(crate) name: String,
    #[serde(rename = "SPDXID")]
    pub(crate) spdx_id: String,
    pub(crate) version_info: Option<String>,
    pub(crate) download_location: String,
    #[serde(default)]
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Relationship {
    pub(crate) spdx_element_id: Option<String>,
    pub(crate) related_spdx_element: Option<String>,
    pub(crate) relationship_type: Option<String>,
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
    let purl = GenericPurl::<String>::from_str(pkg_url)?;

    let package_type = PackageType::from_str(purl.package_type())
        .or_else(|_| type_from_url(&pkg_info.download_location))
        .context(UnknownEcosystem)?;

    // Determine the package name based on its type and namespace.
    let name = formatted_package_name(&package_type, &purl);

    let pkg_version = pkg_info
        .version_info
        .as_ref()
        .ok_or(purl.version())
        .map_err(|_| anyhow!("No version found for `{}`", pkg_info.name))?;

    // Use the qualifiers from the PURL to determine the version details.
    let version = determine_package_version(pkg_version, &purl);

    Ok(Package { name, version, package_type })
}

fn from_locator(registry: &str, locator: &str) -> anyhow::Result<Package> {
    let package_type = PackageType::from_str(registry).map_err(|_| UnknownEcosystem)?;
    let (name, version) = match package_type {
        PackageType::Npm => locator.rsplit_once('@'),
        PackageType::Nuget => locator.rsplit_once('/'),
        PackageType::Maven => locator.rsplit_once(':').filter(|(name, _)| name.contains(':')),
        _ => {
            // Not in the spec, but included for compatibility with our API.
            locator.rsplit_once('@')
        },
    }
    .ok_or(anyhow!("Invalid locator: {}", locator))?;

    let name = decode(name).with_context(|| anyhow!("URL decode failed: {:?}", name))?;

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
        let spdx_info = if let Ok(lock) = serde_json::from_str::<serde_json::Value>(data) {
            serde_json::from_value::<SpdxInfo>(lock)?
        } else if let Ok(lock) = serde_yaml::from_str::<serde_yaml::Value>(data) {
            serde_yaml::from_value::<SpdxInfo>(lock)?
        } else {
            spdx::parse(data).finish().map_err(|e| anyhow!(convert_error(data, e)))?.1
        };

        let spdx_ids: Vec<_> = spdx_info
            .relationships
            .into_iter()
            .filter_map(|r| {
                if r.relationship_type.as_ref().map_or(false, |t| t == "DESCRIBES")
                    && r.spdx_element_id.as_ref().map_or(false, |t| t == &spdx_info.spdx_id)
                {
                    r.related_spdx_element
                } else {
                    None
                }
            })
            .collect();

        let mut packages = Vec::new();
        for package_info in spdx_info.packages {
            if spdx_info.document_describes.contains(&package_info.spdx_id)
                || spdx_ids.contains(&package_info.spdx_id)
            {
                continue;
            }
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
        path.file_name().and_then(OsStr::to_str).map_or(false, |name| {
            name.ends_with(".spdx.json")
                || name.ends_with(".spdx.yaml")
                || name.ends_with(".spdx.yml")
                || name.ends_with(".spdx")
        })
    }

    fn is_path_manifest(&self, _path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
        assert!(error.to_string().contains("version"))
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
        assert!(error.to_string().contains("version"))
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
    fn removes_self_identified_package() {
        let data = r##"SPDXVersion: SPDX-2.2
            DataLicense: CC0-1.0
            SPDXID: SPDXRef-DOCUMENT
            DocumentName: Python-cve-bin-tool
            DocumentNamespace: http://spdx.org/spdxdocs/Python-cve-bin-tool-4137f958-709e-4f44-940e-f477ded25cbd
            LicenseListVersion: 3.22
            Creator: Tool: sbom4python-0.10.4
            Created: 2024-04-01T00:28:13Z
            CreatorComment: <text>This document has been automatically generated.</text>
            DocumentDescribes: SPDXRef-Package1, SPDXRef-Package2
            ##### 

            PackageName: cve-bin-tool
            SPDXID: SPDXRef-Package-1-cve-bin-tool
            PackageVersion: 3.3rc2
            PrimaryPackagePurpose: APPLICATION
            PackageSupplier: Person: Terri Oda (terri.oda@intel.com)
            PackageDownloadLocation: https://pypi.org/project/cve-bin-tool/3.3rc2
            FilesAnalyzed: false
            PackageChecksum: SHA1: c491590aeea36235930d1c6b8480d2489a470ece
            PackageLicenseDeclared: GPL-3.0-or-later
            PackageLicenseConcluded: GPL-3.0-or-later
            PackageCopyrightText: NOASSERTION
            PackageSummary: <text>CVE Binary Checker Tool</text>
            ExternalRef: PACKAGE_MANAGER purl pkg:pypi/cve-bin-tool@3.3rc2
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:terri_oda:cve-bin-tool:3.3rc2:*:*:*:*:*:*:*
            ##### 

            PackageName: aiohttp
            SPDXID: SPDXRef-Package-2-aiohttp
            PackageVersion: 3.9.3
            PrimaryPackagePurpose: LIBRARY
            PackageSupplier: NOASSERTION
            PackageDownloadLocation: https://pypi.org/project/aiohttp/3.9.3
            FilesAnalyzed: false
            PackageLicenseDeclared: NOASSERTION
            PackageLicenseConcluded: Apache-2.0
            PackageLicenseComments: <text>aiohttp declares Apache 2 which is not currently a valid SPDX License identifier or expression.</text>
            PackageCopyrightText: NOASSERTION
            PackageSummary: <text>Async http client/server framework (asyncio)</text>
            ExternalRef: PACKAGE_MANAGER purl pkg:pypi/aiohttp@3.9.3
            #####

            PackageName: @colors/colors
            SPDXID: SPDXRef-Package1
            PackageVersion: 1.5.0
            PackageDownloadLocation: http://github.com/DABH/colors.js.git
            PackageSourceInfo: acquired package info from installed node module manifest file: /usr/local/lib/node_modules/npm/node_modules/@colors/colors/package.json
            PackageOriginator: Person: DABH
            PackageLicenseDeclared: MIT
            PackageLicenseConcluded: MIT
            PackageCopyrightText: NOASSERTION
            PackageHomePage: https://github.com/DABH/colors.js
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@colors\/colors:\@colors\/colors:1.5.0:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:DABH:\@colors\/colors:1.5.0:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:dabh:\@colors\/colors:1.5.0:*:*:*:*:*:*:*
            ExternalRef: PACKAGE-MANAGER purl pkg:npm/%40colors/colors@1.5.0

            PackageName: @discoveryjs/json-ext
            SPDXID: SPDXRef-Package2
            PackageVersion: 0.5.6
            PackageDownloadLocation: NOASSERTION
            PackageSourceInfo: acquired package info from installed node module manifest file: /usr/local/go/src/cmd/vendor/github.com/google/pprof/third_party/d3flamegraph/package-lock.json
            PackageLicenseDeclared: NONE
            PackageLicenseConcluded: NONE
            PackageCopyrightText: NOASSERTION
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@discoveryjs\/json-ext:\@discoveryjs\/json-ext:0.5.6:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@discoveryjs\/json-ext:\@discoveryjs\/json_ext:0.5.6:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@discoveryjs\/json_ext:\@discoveryjs\/json-ext:0.5.6:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@discoveryjs\/json_ext:\@discoveryjs\/json_ext:0.5.6:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@discoveryjs\/json:\@discoveryjs\/json-ext:0.5.6:*:*:*:*:*:*:*
            ExternalRef: SECURITY cpe23Type cpe:2.3:a:\@discoveryjs\/json:\@discoveryjs\/json_ext:0.5.6:*:*:*:*:*:*:*
            ExternalRef: PACKAGE-MANAGER purl pkg:npm/%40discoveryjs/json-ext@0.5.6
            
            Relationship: SPDXRef-DOCUMENT DESCRIBES SPDXRef-Package-1-cve-bin-tool
            Relationship: SPDXRef-Package-1-cve-bin-tool DEPENDS_ON SPDXRef-Package-2-aiohttp
            "##;

        let pkgs = Spdx.parse(data).unwrap();
        assert_eq!(pkgs.len(), 1);

        let expected_pkgs = Package {
            name: "aiohttp".into(),
            version: PackageVersion::FirstParty("3.9.3".into()),
            package_type: PackageType::PyPi,
        };

        assert_eq!(expected_pkgs, pkgs[0]);
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

    #[test]
    fn test_file_type() {
        let parse_results =
            Spdx.parse(include_str!("../../tests/fixtures/appbomination.spdx.json"));
        let expected = anyhow!("Missing package locator for Gradle").to_string();
        let actual = parse_results.err().unwrap().to_string();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_if_lockfile() {
        let test_paths = vec![
            "/foo/bar/test.spdx.json",
            "/foo/bar/test.spdx.yaml",
            "/foo/bar/test.spdx.yml",
            "/foo/bar/test.spdx",
        ];

        for path_str in test_paths {
            let path_buf = PathBuf::from(path_str);
            let is_lockfile = Spdx.is_path_lockfile(&path_buf);
            assert!(is_lockfile, "Failed for path: {}", path_str);
        }
    }
}
