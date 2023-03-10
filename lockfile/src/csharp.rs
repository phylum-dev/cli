use std::ffi::OsStr;
use std::path::Path;

use phylum_types::types::package::PackageType;
use serde::Deserialize;
use serde_xml_rs::Deserializer;

use crate::{Package, PackageVersion, Parse};

pub struct CSProj;

const UTF8_BOM: &str = "\u{feff}";

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PackageReference {
    #[serde(alias = "Include", default)]
    pub name: String,

    #[serde(alias = "Version", default)]
    pub version: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ItemGroup {
    #[serde(alias = "PackageReference", default)]
    pub dependencies: Vec<PackageReference>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Project {
    #[serde(rename = "ItemGroup", default)]
    pub item_groups: Vec<ItemGroup>,
}

impl From<PackageReference> for Package {
    fn from(pkg_ref: PackageReference) -> Self {
        Self {
            name: pkg_ref.name,
            version: PackageVersion::FirstParty(pkg_ref.version),
            package_type: PackageType::Nuget,
        }
    }
}

impl From<Project> for Vec<Package> {
    fn from(proj: Project) -> Self {
        let mut deps = Vec::new();

        for item_group in proj.item_groups {
            if !item_group.dependencies.is_empty() {
                deps.extend(
                    item_group.dependencies.into_iter().map(Package::from).collect::<Vec<_>>(),
                );
            }
        }
        deps
    }
}

impl Parse for CSProj {
    /// Parses `.csproj` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let data = data.trim_start_matches(UTF8_BOM);
        let mut de =
            Deserializer::new_from_reader(data.as_bytes()).non_contiguous_seq_elements(true);
        let parsed = Project::deserialize(&mut de)?;
        Ok(parsed.into())
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.extension() == Some(OsStr::new("csproj"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_csproj() {
        let pkgs = CSProj.parse(include_str!("../../tests/fixtures/sample.csproj")).unwrap();

        assert_eq!(pkgs.len(), 5);
        assert_eq!(pkgs[0].name, "Microsoft.NETFramework.ReferenceAssemblies");
        assert_eq!(pkgs[0].version, PackageVersion::FirstParty("1.0.0".into()));

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "System.ValueTuple");
        assert_eq!(last.version, PackageVersion::FirstParty("4.5.0".into()));
    }

    #[test]
    fn strips_utf8_bom() {
        let pkgs = CSProj.parse(include_str!("../../tests/fixtures/Calculator.csproj")).unwrap();
        assert!(!pkgs.is_empty());
    }
}
