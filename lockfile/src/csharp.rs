use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use anyhow::anyhow;
#[cfg(feature = "generator")]
use lockfile_generator::dotnet::Dotnet as DotnetGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use phylum_types::types::package::PackageType;
use serde::Deserialize;
use serde_xml_rs::Deserializer;

use crate::{Package, PackageVersion, Parse};

const UTF8_BOM: &str = "\u{feff}";

pub struct PackagesLock;

impl Parse for PackagesLock {
    /// Parses `packages.lock.json` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        // Deserialize lockfile as JSON.
        let parsed: PackagesLockJson = serde_json::from_str(data)?;

        // Map all dependencies to their correct package types.
        let packages = parsed
            .dependencies
            .into_iter()
            .flat_map(|(_, deps)| deps.into_iter())
            .map(|(name, dependency)| {
                let version = match (&dependency.dependency_type, &dependency.resolved) {
                    (DependencyType::Project, _) => PackageVersion::Path(None),
                    (_, Some(resolved)) => PackageVersion::FirstParty(resolved.clone()),
                    _ => return Err(anyhow!("invalid dependency {name:?}: {dependency:?}")),
                };

                Ok(Package { version, name, package_type: PackageType::Nuget })
            })
            .collect::<Result<_, _>>()?;

        Ok(packages)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        let file_name = match path.file_name().and_then(|f| f.to_str()) {
            Some(file_name) => file_name,
            None => return false,
        };

        // Accept both `packages.lock.json` and `packages.<project_name>.lock.json`.
        file_name.starts_with("packages.") && file_name.ends_with(".lock.json")
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.extension() == Some(OsStr::new("csproj"))
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&DotnetGenerator)
    }
}

/// `packages.lock.json` structure.
#[derive(Deserialize, Debug)]
struct PackagesLockJson {
    #[serde(rename = "version")]
    _version: usize,
    dependencies: HashMap<String, FrameworkDependencies>,
}

/// `packages.lock.json` .NET framework structure.
type FrameworkDependencies = HashMap<String, Dependency>;

/// `packages.lock.json` dependency structure.
#[derive(Deserialize, Debug)]
struct Dependency {
    #[serde(rename = "type")]
    dependency_type: DependencyType,
    resolved: Option<String>,
}

/// `packages.lock.json` dependency types.
#[derive(Deserialize, Debug)]
enum DependencyType {
    Direct,
    CentralTransitive,
    Transitive,
    Project,
    Build,
    Platform,
    Tool,
}

pub struct CSProj;

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

    fn is_path_manifest(&self, _path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_lock() {
        let pkgs =
            PackagesLock.parse(include_str!("../../tests/fixtures/packages.lock.json")).unwrap();
        assert_eq!(pkgs.len(), 7);

        let expected_pkgs = [
            Package {
                name: "Microsoft.Windows.SDK.Contracts".into(),
                version: PackageVersion::FirstParty("10.0.22621.755".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "SSH.NET".into(),
                version: PackageVersion::FirstParty("2020.0.2".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "example.helpers".into(),
                version: PackageVersion::Path(None),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "Microsoft.SourceLink.GitHub".into(),
                version: PackageVersion::FirstParty("1.1.1".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "Microsoft.Build.Tasks.Git".into(),
                version: PackageVersion::FirstParty("1.1.1".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "System.Buffers".into(),
                version: PackageVersion::FirstParty("4.5.1".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "Microsoft.CodeAnalysis.FxCopAnalyzers".into(),
                version: PackageVersion::FirstParty("3.3.0".into()),
                package_type: PackageType::Nuget,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg), "missing package {expected_pkg:?}");
        }
    }

    #[cfg(feature = "generator")]
    #[test]
    fn lock_generate_csproj() {
        let path = Path::new("../tests/fixtures/lock_generate_csproj/lock_generate_csproj.csproj");
        let lockfile = DotnetGenerator.generate_lockfile(path).unwrap();

        let pkgs = PackagesLock.parse(&lockfile).unwrap();
        assert_eq!(pkgs.len(), 15);

        let expected_pkgs = [
            Package {
                name: "Azure.Core".into(),
                version: PackageVersion::FirstParty("1.34.0".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "Microsoft.Identity.Client".into(),
                version: PackageVersion::FirstParty("4.54.1".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "Serilog".into(),
                version: PackageVersion::FirstParty("3.0.2-dev-02044".into()),
                package_type: PackageType::Nuget,
            },
            Package {
                name: "System.Runtime.CompilerServices.Unsafe".into(),
                version: PackageVersion::FirstParty("6.0.0".into()),
                package_type: PackageType::Nuget,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg), "missing package {expected_pkg:?}");
        }
    }

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
    fn strip_utf8_bom() {
        let pkgs = CSProj.parse(include_str!("../../tests/fixtures/Calculator.csproj")).unwrap();
        assert!(!pkgs.is_empty());
    }
}
