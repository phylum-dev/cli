use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
#[cfg(feature = "generator")]
use lockfile_generator::go::Go as GoGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use nom::error::convert_error;
use nom::Finish;

use crate::parsers::go_sum;
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

#[cfg(test)]
mod tests {
    use phylum_types::types::package::PackageType;

    use super::*;
    use crate::PackageVersion;

    #[test]
    fn parse_go_sum() {
        let pkgs = GoSum.parse(include_str!("../../tests/fixtures/go.sum")).unwrap();
        assert_eq!(pkgs.len(), 1711);

        let expected_pkgs = [
            Package {
                name: "cloud.google.com/go".into(),
                version: PackageVersion::FirstParty("v0.72.0".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "sourcegraph.com/sourcegraph/appdash".into(),
                version: PackageVersion::FirstParty("v0.0.0-20190731080439-ebfcffb1b5c0".into()),
                package_type: PackageType::Golang,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg));
        }
    }

    #[cfg(feature = "generator")]
    #[test]
    fn lock_generate_goproj() {
        let path = Path::new("../tests/fixtures/lock_generate_go/go.mod");
        let lockfile = GoGenerator.generate_lockfile(path).unwrap();

        let mut actual_pkgs = GoSum.parse(&lockfile).unwrap();

        let mut expected_pkgs = [
            // Go.mod direct dependencies.
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
            // Go.mod indirect dependencies.
            Package {
                name: "github.com/mattn/go-colorable".into(),
                version: PackageVersion::FirstParty("v0.1.13".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/mattn/go-isatty".into(),
                version: PackageVersion::FirstParty("v0.0.19".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "golang.org/x/sys".into(),
                version: PackageVersion::FirstParty("v0.12.0".into()),
                package_type: PackageType::Golang,
            },
            // Transitive dependencies.
            Package {
                name: "github.com/coreos/go-systemd/v22".into(),
                version: PackageVersion::FirstParty("v22.5.0".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/godbus/dbus/v5".into(),
                version: PackageVersion::FirstParty("v5.0.4".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/pkg/errors".into(),
                version: PackageVersion::FirstParty("v0.9.1".into()),
                package_type: PackageType::Golang,
            },
            Package {
                name: "github.com/rs/xid".into(),
                version: PackageVersion::FirstParty("v1.5.0".into()),
                package_type: PackageType::Golang,
            },
        ];

        expected_pkgs.sort();
        actual_pkgs.sort();

        assert_eq!(expected_pkgs, *actual_pkgs);
    }
}
