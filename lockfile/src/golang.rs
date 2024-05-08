use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
#[cfg(feature = "generator")]
use lockfile_generator::go::Go as GoGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use nom::error::convert_error;
use nom::Finish;

use crate::parsers::{go_mod, go_sum};
use crate::{Package, Parse};

pub struct GoSum;

impl Parse for GoSum {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let (_, entries) = go_sum::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse go.sum file")?;
        Ok(entries)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("go.sum"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("go.mod"))
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&GoGenerator)
    }
}

pub struct GoDeps {
    pub go: String,
    pub modules: Vec<Package>,
}

fn check_go_directive(version: &str) -> anyhow::Result<()> {
    let mut parts = version.split(|c: char| !c.is_numeric());
    if let (Some(major), Some(minor)) = (parts.next(), parts.next()) {
        let major: u32 = major.parse().unwrap_or(0);
        let minor: u32 = minor.parse().unwrap_or(0);

        // Check if version meets the criteria.
        if major < 1 || (major == 1 && minor < 17) {
            return Err(anyhow!("Minimum supported go directive is 1.17"));
        }
    } else {
        return Err(anyhow!("Error parsing go directive"));
    }
    Ok(())
}

pub struct GoMod;

impl Parse for GoMod {
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let (_, go_mod) = go_mod::parse(data)
            .finish()
            .map_err(|e| anyhow!(e.to_string()))
            .context("Failed to parse go.mod file")?;

        check_go_directive(&go_mod.go)?;

        Ok(go_mod.modules)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("go.mod"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("go.mod"))
    }
}

#[cfg(test)]
mod tests {
    use phylum_types::types::package::PackageType;

    use super::*;
    use crate::PackageVersion;

    #[test]
    fn parse_go_sum() {
        let pkgs = GoSum.parse(include_str!("../../tests/fixtures/go.sum")).unwrap();
        assert_eq!(pkgs.len(), 674);

        let expected_pkgs = [
            Package {
                name: "cloud.google.com/go".into(),
                version: PackageVersion::FirstParty("v0.72.0".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "sigs.k8s.io/yaml".into(),
                version: PackageVersion::FirstParty("v1.2.0".into()),
                package_type: PackageType::Golang,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_go_mod() {
        let mut pkgs = GoMod.parse(include_str!("../../tests/fixtures/go.mod")).unwrap();
        pkgs.sort();

        let expected_pkgs = [
            Package {
                name: "../replacedmodule".into(),
                version: PackageVersion::Path(Some("../replacedmodule".into())),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/newmodule".into(),
                version: PackageVersion::FirstParty("v3.2.1".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/newmodule".into(),
                version: PackageVersion::FirstParty("v3.2.2".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/newmodule".into(),
                version: PackageVersion::FirstParty("v3.2.3".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/othermodule".into(),
                version: PackageVersion::FirstParty("v1.2.3".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/go-chi/chi/v5".into(),
                version: PackageVersion::FirstParty("v5.0.12".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/mattn/go-colorable".into(),
                version: PackageVersion::FirstParty("v0.1.13".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/mattn/go-isatty".into(),
                version: PackageVersion::FirstParty("v0.0.20".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/rs/zerolog".into(),
                version: PackageVersion::FirstParty("v1.32.0".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "golang.org/x/sys".into(),
                version: PackageVersion::FirstParty("v0.12.0".into()),
                package_type: PackageType::Golang,
            },
        ];

        assert_eq!(expected_pkgs, *pkgs)
    }

    #[test]
    fn parse_go_mod_unsupported() {
        let go_mod_content = r#"
            module cli/example
            
            go 1.14
            
            require (
                github.com/go-chi/chi/v5 v5.0.12
                github.com/rs/zerolog v1.32.0
            )
            
            require (
                github.com/mattn/go-colorable v0.1.13 // indirect
                github.com/mattn/go-isatty v0.0.20 // indirect
                golang.org/x/sys v0.12.0 // indirect
            )
        "#;

        let error = GoMod.parse(go_mod_content).err().unwrap();
        assert_eq!(error.to_string(), "Minimum supported go directive is 1.17")
    }
}
