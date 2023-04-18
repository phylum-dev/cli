use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
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
}
