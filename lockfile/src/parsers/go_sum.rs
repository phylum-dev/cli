use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{alphanumeric1, line_ending, space0, space1};
use nom::combinator::{opt, recognize};
use nom::multi::many1;
use nom::sequence::{preceded, tuple};

use super::Result;
use crate::{Package, PackageVersion};

pub fn parse(input: &str) -> Result<&str, Vec<Package>> {
    let (input, pkg_options) = many1(package)(input)?;
    let pkgs = pkg_options.iter().flatten().cloned().collect::<Vec<_>>();

    Ok((input, pkgs))
}

fn package(input: &str) -> Result<&str, Option<Package>> {
    let (input, name) = package_name(input)?;
    let (input, version) = package_version(input)?;
    let (input, _hash) = package_hash(input)?;

    let package = version.map(|version| Package {
        name: name.to_string(),
        version: PackageVersion::FirstParty(version.to_string()),
    });

    Ok((input, package))
}

fn package_name(input: &str) -> Result<&str, &str> {
    // Take away any leading whitespace.
    let (input, _) = space0(input)?;

    // The package name will be everything up until a space.
    recognize(take_until(" "))(input)
}

fn package_version(input: &str) -> Result<&str, Option<&str>> {
    // Take away any leading whitespace.
    let (input, _) = space0(input)?;

    // Accept all of `v[a-zA-Z0-9.+-]+` as valid version characters.
    let (input, version) = recognize(tuple((
        tag("v"),
        many1(alt((alphanumeric1, tag("."), tag("-"), tag("+")))),
    )))(input)?;

    // Check for version suffix.
    let (input, version_suffix) = opt(tag("/go.mod"))(input)?;

    // Expect at least one whitespace after version.
    let (input, _) = space1(input)?;

    // If the package version ends in "/go.mod" then this entry
    // just records the hash for a go.mod file. The package this
    // go.mod file belongs to will also be listed, and that's what
    // we're interested in, so we simply discard this entry.
    match version_suffix {
        Some(_) => Ok((input, None)),
        None => Ok((input, Some(version))),
    }
}

fn package_hash(input: &str) -> Result<&str, &str> {
    // Take away any leading whitespace.
    let (input, _) = space0(input)?;

    // Base64 parser for package hash.
    let base64_parser = recognize(many1(alt((alphanumeric1, tag("+"), tag("/"), tag("=")))));

    // Parse base64 hash with `h1:` prefix.
    let (input, hash) = preceded(tag("h1:"), base64_parser)(input)?;

    // Expect EOL.
    let (input, _) = line_ending(input)?;

    Ok((input, hash))
}
