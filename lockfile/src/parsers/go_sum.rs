use std::ops::Not;

use nom::bytes::complete::take_until;
use nom::character::complete::space0;
use nom::combinator::recognize;
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::Result;

pub fn parse(input: &str) -> Result<&str, Vec<PackageDescriptor>> {
    let pkgs = input.lines().filter_map(package).collect::<Vec<_>>();
    Ok((input, pkgs))
}

fn package(input: &str) -> Option<PackageDescriptor> {
    let (input, name) = package_name(input).ok()?;
    let (input, version) = package_version(input).ok()?;
    let (_, _hash) = package_hash(input).ok()?;

    // If the package version ends in "/go.mod" then this entry
    // just records the hash for a go.mod file. The package this
    // go.mod file belongs to will also be listed, and that's what
    // we're interested in, so we simply discard this entry.

    version.ends_with(r"/go.mod").not().then(|| PackageDescriptor {
        name: name.to_string(),
        version: version.to_string(),
        package_type: PackageType::Golang,
    })
}

fn package_name(input: &str) -> Result<&str, &str> {
    // take away any leading whitespace
    let (input, _) = recognize(space0)(input)?;

    // the package name will be everything up until a space
    recognize(take_until(" "))(input)
}

fn package_version(input: &str) -> Result<&str, &str> {
    // take away the leading whitespace
    let (input, _) = space0(input)?;

    // the version will be the string up until a space
    recognize(take_until(" "))(input)
}

fn package_hash(input: &str) -> Result<&str, &str> {
    // take away the leading whitespace, then the hash is
    // everything left
    let (hash, _) = space0(input)?;
    Ok(("", hash))
}
