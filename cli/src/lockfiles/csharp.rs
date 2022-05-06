use serde::Deserialize;
use serde_xml_rs::Deserializer;
use std::io;
use std::path::Path;

use phylum_types::types::package::{PackageDescriptor, PackageType};

use crate::lockfiles::{ParseResult, Parseable};

pub struct CSProj(String);

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

impl Parseable for CSProj {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(CSProj(
            std::fs::read_to_string(filename)?
                .trim_start_matches(INVALID_CHAR)
                .to_string(),
        ))
    }

    /// Parses `.csproj` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let mut de =
            Deserializer::new_from_reader(self.0.as_bytes()).non_contiguous_seq_elements(true);
        let parsed = Project::deserialize(&mut de)?;
        Ok(parsed.into())
    }

    fn package_type() -> PackageType {
        PackageType::Nuget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_csproj() {
        let parser = CSProj::new(Path::new("tests/fixtures/sample.csproj")).unwrap();

        let pkgs = parser.parse().unwrap();
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
        let parser = CSProj::new(Path::new("tests/fixtures/Calculator.csproj")).unwrap();
        let pkgs = parser.parse().unwrap();
        assert!(!pkgs.is_empty());
    }
}
