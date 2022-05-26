use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::types::package::PackageType;

use super::parsers::gem;
use crate::lockfiles::{Parse, ParseResult};

pub struct GemLock;

impl Parse for GemLock {
    /// Parses `Gemfile.lock` files into a vec of packages
    fn parse(&self, data: &str) -> ParseResult {
        let (_, entries) = gem::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse gem lock file")?;
        Ok(entries)
    }

    fn package_type(&self) -> PackageType {
        PackageType::RubyGems
    }
}

#[cfg(test)]
mod tests {
    use phylum_types::types::package::PackageType;

    use super::*;

    #[test]
    fn lock_parse_gem() {
        let pkgs = GemLock.parse_file("tests/fixtures/Gemfile.lock").unwrap();
        assert_eq!(pkgs.len(), 214);
        assert_eq!(pkgs[0].name, "CFPropertyList");
        assert_eq!(pkgs[0].version, "2.3.6");
        assert_eq!(pkgs[0].package_type, PackageType::RubyGems);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "xpath");
        assert_eq!(last.version, "3.2.0");
        assert_eq!(last.package_type, PackageType::RubyGems);
    }
}
