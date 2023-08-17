use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
#[cfg(feature = "generator")]
use lockfile_generator::bundler::Bundler as BundlerGenerator;
#[cfg(feature = "generator")]
use lockfile_generator::Generator;
use nom::error::convert_error;
use nom::Finish;

use super::parsers::gem;
use crate::{Package, Parse};

pub struct GemLock;

impl Parse for GemLock {
    /// Parses `Gemfile.lock` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let (_, entries) = gem::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse gem lock file")?;
        Ok(entries)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Gemfile.lock")) && path.is_file()
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("Gemfile")) && path.is_file()
    }

    #[cfg(feature = "generator")]
    fn generator(&self) -> Option<&'static dyn Generator> {
        Some(&BundlerGenerator)
    }
}

#[cfg(test)]
mod tests {
    use phylum_types::types::package::PackageType;

    use super::*;
    use crate::{PackageVersion, ThirdPartyVersion};

    #[test]
    fn lock_parse_gem() {
        let pkgs = GemLock.parse(include_str!("../../tests/fixtures/Gemfile.lock")).unwrap();
        assert_eq!(pkgs.len(), 11);

        let expected_pkgs = [
            Package {
                name: "yaml".into(),
                version: PackageVersion::Git("git@github.com:ruby/yaml.git#b89ff5a79c2abbf81612ffe9a6c184db347365c9".into()),
                package_type: PackageType::RubyGems,
            },
            Package {
                name: "main".into(),
                version: PackageVersion::Git("https://gist.github.com/cd-work/bb850193cd4d1eff0d7021c9a3899882.git#24b38dc61f9e2ee241e1f5eba4fdba4b5ed1e737".into()),
                package_type: PackageType::RubyGems,
            },
            Package {
                name: "benchmark".into(),
                version: PackageVersion::Git("https://github.com/ruby/benchmark.git#303ac8f28b9aad6abe95c86bc64ea891f77ac93e".into()),
                package_type: PackageType::RubyGems,
            },
            Package {
                name: "csv".into(),
                version: PackageVersion::Path(Some("/tmp/csv".into())),
                package_type: PackageType::RubyGems,
            },
            Package {
                name: "wirble".into(),
                version: PackageVersion::FirstParty("0.1.3".into()),
                package_type: PackageType::RubyGems,
            },
            Package {
                name: "rspec-mocks".into(),
                version: PackageVersion::ThirdParty(ThirdPartyVersion {
                    registry: "http://rubygems.org/".into(),
                    version: "3.11.2".into(),
                }),
                package_type: PackageType::RubyGems,
            },
        ];

        for expected_pkg in expected_pkgs {
            assert!(pkgs.contains(&expected_pkg), "missing package {expected_pkg:?}");
        }
    }

    #[test]
    fn empty_lockfile() {
        let pkgs = GemLock.parse(include_str!("../../tests/fixtures/Gemfile.empty.lock")).unwrap();
        assert!(pkgs.is_empty());
    }
}
