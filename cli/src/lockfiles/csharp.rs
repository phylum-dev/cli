use serde::Deserialize;
use serde_xml_rs::Deserializer;

use phylum_types::types::package::{PackageDescriptor, PackageType};

use crate::lockfiles::{ParseResult, Parser};

pub struct CSProj;

const INVALID_CHAR: &str = "\u{feff}";

#[derive(Debug, Deserialize, PartialEq)]
pub struct PackageReference {
    #[serde(alias = "Include", default)]
    pub name: String,

    #[serde(alias = "Version", default)]
    pub version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ItemGroup {
    #[serde(alias = "PackageReference", default)]
    pub dependencies: Vec<PackageReference>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Project {
    #[serde(rename = "ItemGroup", default)]
    pub item_groups: Vec<ItemGroup>,
}

impl From<PackageReference> for PackageDescriptor {
    fn from(pkg_ref: PackageReference) -> Self {
        PackageDescriptor {
            name: pkg_ref.name,
            version: pkg_ref.version,
            package_type: PackageType::Nuget,
        }
    }
}

impl From<Project> for Vec<PackageDescriptor> {
    fn from(proj: Project) -> Self {
        let mut deps = Vec::new();

        for item_group in proj.item_groups {
            if !item_group.dependencies.is_empty() {
                deps.extend(
                    item_group
                        .dependencies
                        .into_iter()
                        .map(PackageDescriptor::from)
                        .collect::<Vec<_>>(),
                );
            }
        }
        deps
    }
}

impl Parser for CSProj {
    /// Parses `.csproj` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let data = data.trim_start_matches(INVALID_CHAR);
        let mut de =
            Deserializer::new_from_reader(data.as_bytes()).non_contiguous_seq_elements(true);
        let parsed = Project::deserialize(&mut de)?;
        Ok(parsed.into())
    }

    fn package_type(&self) -> PackageType {
        PackageType::Nuget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfiles::parse_file;

    #[test]
    fn lock_parse_csproj() {
        let pkgs = parse_file(CSProj, "tests/fixtures/sample.csproj").unwrap();

        assert_eq!(pkgs.len(), 5);
        assert_eq!(pkgs[0].name, "Microsoft.NETFramework.ReferenceAssemblies");
        assert_eq!(pkgs[0].version, "1.0.0");
        assert_eq!(pkgs[0].package_type, PackageType::Nuget);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "System.ValueTuple");
        assert_eq!(last.version, "4.5.0");
        assert_eq!(last.package_type, PackageType::Nuget);
    }

    #[test]
    fn lock_parse_another_invalid_char() {
        let pkgs = parse_file(CSProj, "tests/fixtures/Calculator.csproj").unwrap();
        assert!(!pkgs.is_empty());
    }
}
