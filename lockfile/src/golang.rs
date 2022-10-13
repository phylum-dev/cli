use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::PackageType;

use crate::parsers::go_sum;
use crate::{Parse, ParseResult};

pub struct GoSum;

impl Parse for GoSum {
    fn parse(&self, data: &str) -> ParseResult {
        let (_, entries) = go_sum::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse go.sum file")?;
        Ok(entries)
    }

    fn package_type(&self) -> PackageType {
        PackageType::Golang
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("go.sum"))
    }
}

#[cfg(test)]
mod tests {
    use phylum_types::types::package::PackageType;

    use super::*;

    #[test]
    fn parse_go_sum() {
        let pkgs = GoSum.parse(include_str!("../../tests/fixtures/go.sum")).unwrap();
        assert_eq!(pkgs.len(), 675);

        // check the first package in the example go.sum
        assert_eq!(pkgs[0].name, "cloud.google.com/go");
        assert_eq!(pkgs[0].version, "v0.72.0");
        assert_eq!(pkgs[0].package_type, PackageType::Golang);

        // check the last package in the example go.sum
        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "sourcegraph.com/sourcegraph/appdash");
        assert_eq!(last.version, "v0.0.0-20190731080439-ebfcffb1b5c0");
        assert_eq!(last.package_type, PackageType::Golang);
    }
}