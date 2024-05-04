use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, bail, Context};
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
    let numeric_part = version.split(|c: char| !c.is_numeric()).collect::<Vec<&str>>();
    if let (Some(major), Some(minor)) = (numeric_part.first(), numeric_part.get(1)) {
        let major: u32 = major.parse().unwrap_or(0);
        let minor: u32 = minor.parse().unwrap_or(0);

        // Check if version meets the criteria.
        if major < 1 || minor < 17 {
            bail!("Minimum supported go directive is 1.17")
        }
    } else {
        bail!("Error parsing go directive")
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
        let pkgs = GoMod.parse(include_str!("../../tests/fixtures/go.mod")).unwrap();
        assert_eq!(pkgs.len(), 5);

        let expected_pkgs = [
            Package {
                name: "github.com/go-chi/chi/v5".into(),
                version: PackageVersion::FirstParty("v5.0.12".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/rs/zerolog".into(),
                version: PackageVersion::FirstParty("v1.32.0".into()),
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
                name: "golang.org/x/sys".into(),
                version: PackageVersion::FirstParty("v0.12.0".into()),
                package_type: PackageType::Golang,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
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

    #[test]
    fn parse_go_mod_replace() {
        let go_mod_content = r#"
            module cli/example
            
            go 1.17

            require (
                example.com/othermodule v1.2.3
                example.com/thismodule v1.2.3
                example.com/thatmodule v1.2.3
            )

            replace example.com/thatmodule => ../thatmodule
            replace example.com/thismodule v1.2.3 => example.com/newmodule v3.2.1
        "#;

        let pkgs = GoMod.parse(go_mod_content).unwrap();
        assert_eq!(pkgs.len(), 3);

        let expected_pkgs = [
            Package {
                name: "example.com/othermodule".into(),
                version: PackageVersion::FirstParty("v1.2.3".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/newmodule".into(),
                version: PackageVersion::FirstParty("v3.2.1".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "../thatmodule".into(),
                version: PackageVersion::Path(Some("../thatmodule".into())),
                package_type: PackageType::Golang,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_go_mod_replace_block() {
        let go_mod_content = r#"
            module cli/example
            
            go 1.17

            require (
                example.com/othermodule v1.2.3
                example.com/thismodule v1.2.3
                example.com/thatmodule v1.2.3
            )

            replace (
                example.com/thatmodule => ../thatmodule
                example.com/thismodule v1.2.3 => example.com/newmodule v3.2.1
            )
        "#;

        let pkgs = GoMod.parse(go_mod_content).unwrap();
        assert_eq!(pkgs.len(), 3);

        let expected_pkgs = [
            Package {
                name: "example.com/othermodule".into(),
                version: PackageVersion::FirstParty("v1.2.3".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/newmodule".into(),
                version: PackageVersion::FirstParty("v3.2.1".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "../thatmodule".into(),
                version: PackageVersion::Path(Some("../thatmodule".into())),
                package_type: PackageType::Golang,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_go_mod_replace_block_indirect() {
        let go_mod_content = r#"
            module cli/example
            
            go 1.17

            require (
                example.com/othermodule v1.2.3
                example.com/thismodule v1.2.3
            )

            require (
                example.com/othermodule v1.2.3 // indirect
            )

            replace (
                example.com/othermodule v1.2.3 => example.com/newmodule v3.2.1
            )
        "#;

        let pkgs = GoMod.parse(go_mod_content).unwrap();
        assert_eq!(pkgs.len(), 3);

        let expected_pkgs = [
            Package {
                name: "example.com/othermodule".into(),
                version: PackageVersion::FirstParty("v1.2.3".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/thismodule".into(),
                version: PackageVersion::FirstParty("v1.2.3".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "example.com/newmodule".into(),
                version: PackageVersion::FirstParty("v3.2.1".into()),
                package_type: PackageType::Golang,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[test]
    fn parse_go_mod_exclude() {
        let go_mod_content = r#"
            module cli/example
            
            go 1.17

            require (
                example.com/othermodule v1.2.3
                example.com/thismodule v1.2.3
                example.com/thatmodule v1.2.3
            )

            exclude example.com/thismodule v1.2.3
            exclude example.com/thatmodule v1.2.3
        "#;

        let pkgs = GoMod.parse(go_mod_content).unwrap();

        let expected_pkgs = [Package {
            name: "example.com/othermodule".into(),
            version: PackageVersion::FirstParty("v1.2.3".into()),
            package_type: PackageType::Golang,
        }];

        assert_eq!(expected_pkgs, *pkgs)
    }

    #[test]
    fn parse_go_mod_exclude_block() {
        let go_mod_content = r#"
            module cli/example
            
            go 1.17

            require (
                example.com/othermodule v1.2.3
                example.com/thismodule v1.2.3
                example.com/thatmodule v1.2.3
            )

            exclude (
                example.com/thismodule v1.2.3
                example.com/thatmodule v1.2.3
            )
        "#;

        let pkgs = GoMod.parse(go_mod_content).unwrap();

        let expected_pkgs = [Package {
            name: "example.com/othermodule".into(),
            version: PackageVersion::FirstParty("v1.2.3".into()),
            package_type: PackageType::Golang,
        }];

        assert_eq!(expected_pkgs, *pkgs)
    }
}
