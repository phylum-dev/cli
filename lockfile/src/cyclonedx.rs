use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;

use anyhow::anyhow;
use phylum_types::types::package::PackageType;
use purl::GenericPurl;
use serde::Deserialize;

use crate::{determine_package_version, formatted_package_name, Package, Parse, UnknownEcosystem};

/// Define the generic trait for components.
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

/// CycloneDX BOM.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bom<T> {
    components: Option<T>,
}

/// Struct for wrapping a list of components from XML.
#[derive(Clone, Debug, Deserialize)]
struct Components<T> {
    #[serde(rename = "component")]
    components: Vec<T>,
}

/// Represents a single XML component.
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

    fn components(&self) -> Option<&[Self]> {
        self.components.as_ref().map(|comps| comps.components.as_slice())
    }
}

/// Represents a single JSON component.
#[derive(Clone, Debug, Deserialize)]
struct JsonComponent {
    #[serde(rename = "type")]
    component_type: String,
    name: String,
    version: String,
    scope: Option<String>,
    purl: Option<String>,
    #[serde(default)]
    components: Vec<JsonComponent>,
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

    fn components(&self) -> Option<&[Self]> {
        Some(&self.components)
    }
}

/// Filter components based on the type and scope.
fn filter_components<T: Component>(components: &[T]) -> impl Iterator<Item = &'_ T> {
    components
        .iter()
        .filter(|&comp| {
            let type_check = comp.component_type() == "application"
                || comp.component_type() == "framework"
                || comp.component_type() == "library";

            // The scope is optional and can be required, optional, or excluded
            // If the scope is None, the spec implies required
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
            std::iter::once(comp).chain(nested_iter)
        })
}

/// Convert a component's package URL (PURL) into a package object.
fn from_purl<T: Component>(component: &T) -> anyhow::Result<Package> {
    let purl_str = component
        .purl()
        .ok_or_else(|| anyhow!("Missing purl for {}:{}", component.name(), component.version()))?;
    let purl = GenericPurl::<String>::from_str(purl_str)?;
    let package_type = PackageType::from_str(purl.package_type()).map_err(|_| UnknownEcosystem)?;

    // Determine the package name based on its type and namespace.
    let name = formatted_package_name(&package_type, &purl);

    // Extract the package version
    let pkg_version = purl
        .version()
        .ok_or(&component.version())
        .map_err(|_| anyhow!("No version found for `{}`", name))?;

    // Use the qualifiers from the PURL to determine the version details.
    let version = determine_package_version(pkg_version, &purl);

    Ok(Package { name, version, package_type })
}

pub struct CycloneDX;

impl CycloneDX {
    fn process_components<T: Component>(components: Option<&[T]>) -> anyhow::Result<Vec<Package>> {
        let comp = components.unwrap_or_default();
        let packages = filter_components(comp)
            .map(from_purl)
            .filter(|r| !r.as_ref().is_err_and(|e| e.is::<UnknownEcosystem>()))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(packages)
    }
}

impl Parse for CycloneDX {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        if let Ok(lock) = serde_json::from_str::<serde_json::Value>(data) {
            let parsed: Bom<Vec<JsonComponent>> = serde_json::from_value(lock)?;
            Self::process_components(parsed.components.as_deref())
        } else {
            let parsed: Bom<Components<XmlComponent>> = serde_xml_rs::from_str(data)?;
            let components = parsed.components.map(|c| c.components);
            Self::process_components(components.as_deref())
        }
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(OsStr::to_str)
            .map_or(false, |name| name.ends_with("bom.json") || name.ends_with("bom.xml"))
    }

    fn is_path_manifest(&self, _path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::PackageVersion;

    #[test]
    fn parse_cyclonedx_nested_components() {
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

        let pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/nested_bom.json")).unwrap();
        assert_eq!(pkgs, expected_pkgs);
    }

    #[test]
    fn parse_cyclonedx_1_5() {
        let json_pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/bom.1.5.json")).unwrap();
        let xml_pkgs = CycloneDX.parse(include_str!("../../tests/fixtures/bom.1.5.xml")).unwrap();
        assert_eq!(json_pkgs.len(), xml_pkgs.len());
        assert_eq!(json_pkgs, xml_pkgs);
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

    #[test]
    fn test_if_lockfile() {
        let test_paths = vec![
            "/foo/bar/test.bom.json",
            "/foo/bar/test.bom.xml",
            "/foo/bar/bom.json",
            "/foo/bar/bom.xml",
        ];

        for path_str in test_paths {
            let path_buf = PathBuf::from(path_str);
            let is_lockfile = CycloneDX.is_path_lockfile(&path_buf);
            assert!(is_lockfile, "Failed for path: {}", path_str);
        }
    }

    #[test]
    fn test_ignore_unsupported_ecosystem() {
        let ignored_component = JsonComponent {
            component_type: "library".into(),
            name: "adduser".into(),
            version: "3.118ubuntu5".into(),
            scope: None,
            purl: Some("pkg:deb/ubuntu/adduser@3.118ubuntu5?arch=all&distro=ubuntu-22.04".into()),
            components: vec![],
        };

        let component = JsonComponent {
            component_type: "library".into(),
            name: "abbrev".into(),
            version: "1.1.1".into(),
            scope: None,
            purl: Some("pkg:npm/abbrev@1.1.1".into()),
            components: vec![],
        };

        let expected_package = Package {
            name: "abbrev".into(),
            version: PackageVersion::FirstParty("1.1.1".into()),
            package_type: PackageType::Npm,
        };

        let bom: Bom<Vec<JsonComponent>> =
            Bom { components: Some(vec![component, ignored_component]) };

        let packages = CycloneDX::process_components(bom.components.as_deref()).unwrap();

        assert!(packages.len() == 1);
        assert_eq!(packages[0], expected_package);
    }
}
