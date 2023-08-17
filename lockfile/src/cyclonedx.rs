use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;

use anyhow::anyhow;
use phylum_types::types::package::PackageType;
use purl::GenericPurl;
use serde::Deserialize;
use thiserror::Error;

use crate::{Package, PackageVersion, Parse, ThirdPartyVersion};

// Define a custom error for unknown ecosystems.
#[derive(Error, Debug)]
#[error("Could not determine ecosystem")]
struct UnknownEcosystem;

// Define the generic trait for components.
trait Component {
    fn component_type(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn scope(&self) -> Option<&str>;
    fn purl(&self) -> Option<&str>;
    fn components(&self) -> Option<&[Self]>
    where
        Self: Sized;
}

// CycloneDX BOM.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bom<T> {
    components: Option<T>,
}

// Struct for wrapping a list of components from XML.
#[derive(Clone, Debug, Deserialize)]
struct Components<T> {
    #[serde(rename = "component")]
    components: Vec<T>,
}

// Represents a single XML component.
#[derive(Clone, Debug, Deserialize)]
struct XmlComponent {
    #[serde(rename = "type")]
    component_type: String,
    name: String,
    version: String,
    scope: Option<String>,
    purl: Option<String>,
    components: Option<Components<XmlComponent>>,
}

impl Component for XmlComponent {
    fn component_type(&self) -> &str {
        &self.component_type
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    fn purl(&self) -> Option<&str> {
        self.purl.as_deref()
    }

    fn components(&self) -> Option<&[Self]>
    where
        Self: Sized,
    {
        self.components.as_ref().map(|comps| comps.components.as_slice())
    }
}

// Represents a single JSON component.
#[derive(Clone, Debug, Deserialize)]
struct JsonComponent {
    #[serde(rename = "type")]
    component_type: String,
    name: String,
    version: String,
    scope: Option<String>,
    purl: Option<String>,
    components: Option<Vec<JsonComponent>>,
}

impl Component for JsonComponent {
    fn component_type(&self) -> &str {
        &self.component_type
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    fn purl(&self) -> Option<&str> {
        self.purl.as_deref()
    }

    fn components(&self) -> Option<&[Self]>
    where
        Self: Sized,
    {
        self.components.as_deref()
    }
}

// Filter components based on their type and scope.
fn filter_components<T: Component>(components: &[T]) -> impl Iterator<Item = &'_ T> {
    components
        .iter()
        .filter(|&comp| {
            let type_check = comp.component_type() == "application"
                || comp.component_type() == "framework"
                || comp.component_type() == "library";

            // Check if the scope is "required" or not specified (required)
            let scope_check = match comp.scope() {
                Some(scope) => scope == "required",
                None => true,
            };

            type_check && scope_check
        })
        .flat_map(|comp| {
            let nested_iter = match comp.components() {
                Some(nested) => filter_components(nested).collect::<Vec<_>>(),
                None => Vec::new(),
            };
            std::iter::once(comp).chain(nested_iter.into_iter())
        })
}

// Convert a component's Package URL (PURL) into a Package object.
fn from_purl<T: Component>(component: &T) -> anyhow::Result<Package> {
    let purl_str = component
        .purl()
        .ok_or_else(|| anyhow!("Missing purl for {}:{}", component.name(), component.version()))?;
    let purl = GenericPurl::<String>::from_str(purl_str)?;
    let package_type = PackageType::from_str(purl.package_type()).map_err(|_| UnknownEcosystem)?;

    // Determine the package name based on its type and namespace.
    let name = match (package_type, purl.namespace()) {
        (PackageType::Maven, Some(ns)) => format!("{}:{}", ns, purl.name()),
        (PackageType::Npm | PackageType::Golang, Some(ns)) => format!("{}/{}", ns, purl.name()),
        _ => purl.name().into(),
    };

    // Extract the package version
    let pkg_version = purl
        .version()
        .ok_or(&component.version())
        .map_err(|_| anyhow!("No version found for `{}`", name))?;

    // Use the qualifiers from the PURL to determine the version details.
    let version = purl
        .qualifiers()
        .iter()
        .find_map(|(key, value)| match key.as_ref() {
            "repository_url" => Some(PackageVersion::ThirdParty(ThirdPartyVersion {
                version: pkg_version.into(),
                registry: value.to_string(),
            })),
            "download_url" => Some(PackageVersion::DownloadUrl(value.to_string())),
            "vcs_url" => {
                if value.starts_with("git+") {
                    Some(PackageVersion::Git(value.to_string()))
                } else {
                    None
                }
            },
            _ => None,
        })
        .unwrap_or(PackageVersion::FirstParty(pkg_version.into()));

    Ok(Package { name, version, package_type })
}

pub struct CycloneDX;

impl Parse for CycloneDX {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        if let Ok(lock) = serde_json::from_str::<serde_json::Value>(data) {
            let parsed: Bom<Vec<JsonComponent>> = serde_json::from_value(lock)?;
            parsed.components.map_or(Ok(vec![]), |comp| {
                let component_iter = filter_components(&comp);
                component_iter.map(from_purl).collect()
            })
        } else {
            let parsed: Bom<Components<XmlComponent>> = serde_xml_rs::from_str(data)?;
            parsed.components.map_or(Ok(vec![]), |comp| {
                let component_iter = filter_components(&comp.components);
                component_iter.map(from_purl).collect()
            })
        }
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("bom.json"))
            || path.file_name() == Some(OsStr::new("bom.xml"))
    }

    fn is_path_manifest(&self, _path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_cyclonedx_1_5_json() {
        let sample_data = r#"
        {
            "bomFormat": "CycloneDX",
            "specVersion": "1.5",
            "components": [
                {
                    "type": "framework",
                    "name": "FrameworkA",
                    "version": "1.0",
                    "scope": "required",
                    "purl": "pkg:npm/FrameworkA@1.0",
                    "components": [
                        {
                            "type": "library",
                            "name": "LibA",
                            "version": "1.1",
                            "scope": "required",
                            "purl": "pkg:npm/LibA@1.1"
                        },
                        {
                            "type": "library",
                            "name": "LibB",
                            "version": "1.2",
                            "purl": "pkg:pypi/LibB@1.2"
                        }
                    ]
                },
                {
                    "type": "application",
                    "name": "AppA",
                    "version": "1.0",
                    "scope": "required",
                    "purl": "pkg:pypi/AppA@1.0"
                }
            ]
        }
        "#;

        let expected_pkgs = vec![
            Package {
                name: "FrameworkA".into(),
                version: PackageVersion::FirstParty("1.0".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "LibA".into(),
                version: PackageVersion::FirstParty("1.1".into()),
                package_type: PackageType::Npm,
            },
            Package {
                name: "LibB".into(),
                version: PackageVersion::FirstParty("1.2".into()),
                package_type: PackageType::PyPi,
            },
            Package {
                name: "AppA".into(),
                version: PackageVersion::FirstParty("1.0".into()),
                package_type: PackageType::PyPi,
            },
        ];

        let pkgs = CycloneDX.parse(sample_data).unwrap();
        assert_eq!(pkgs, expected_pkgs);
    }

    #[test]
    fn parse_cyclonedx_1_4() {
        let json_pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/bom.json")).unwrap();
        let xml_pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/bom.xml")).unwrap();
        assert_eq!(json_pkgs.len(), xml_pkgs.len());
        assert_eq!(json_pkgs, xml_pkgs);
    }

    #[test]
    fn parse_cyclonedx_1_3() {
        let json_pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/bom.1.3.json")).unwrap();
        let xml_pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/bom.1.3.xml")).unwrap();
        assert_eq!(json_pkgs.len(), xml_pkgs.len());
        assert_eq!(json_pkgs, xml_pkgs);
    }
}
