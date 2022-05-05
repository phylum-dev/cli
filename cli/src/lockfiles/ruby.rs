use std::io;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;

use super::parsers::gem;
use crate::lockfiles::{ParseResult, Parseable};

pub struct GemLock(String);

impl Parseable for GemLock {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(GemLock(std::fs::read_to_string(filename)?))
    }

    /// Parses `Gemfile.lock` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let data = self.0.as_str();
        let (_, entries) = gem::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse gem lock file")?;
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phylum_types::types::package::PackageType;

    #[test]
    fn lock_parse_gem() {
        let parser = GemLock::new(Path::new("tests/fixtures/Gemfile.lock")).unwrap();

        let pkgs = parser.parse().unwrap();
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
