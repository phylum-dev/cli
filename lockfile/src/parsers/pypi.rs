use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::character::complete::{alphanumeric1, char, not_line_ending, space0};
use nom::combinator::{eof, opt, recognize, rest, verify};
use nom::multi::{many0, many1, separated_list0};
use nom::sequence::{delimited, pair, terminated};

use crate::parsers::Result;
use crate::{Package, PackageVersion};

pub fn parse(input: &str) -> Result<&str, Vec<Package>> {
    let mut pkgs = Vec::new();

    // Iterate over all non-empty lines.
    for line in input.lines().filter(|line| !line.starts_with('#') && !line.trim().is_empty()) {
        let (_, pkg) = package(line)?;
        pkgs.push(pkg);
    }

    Ok((input, pkgs))
}

fn package(input: &str) -> Result<&str, Package> {
    // Ignore everything after `;`.
    let (_, input) = recognize(alt((take_until(";"), not_line_ending)))(input)?;

    // Parse for `-e` dependencies.
    if let Ok(editable) = editable(input) {
        return Ok(editable);
    }

    let (input, name) = package_name(input)?;
    let name = name.trim().to_string();

    // Parse URI versions like files/git/etc.
    if let Ok((input, uri_version)) = uri_version(input) {
        // Ensure line is empty after the dependency.
        line_done(input)?;

        let version = if uri_version.starts_with("file:") {
            PackageVersion::Path(Some(uri_version.into()))
        } else if uri_version.starts_with("git+") {
            PackageVersion::Git(uri_version.into())
        } else {
            PackageVersion::Internet(uri_version.into())
        };

        return Ok((input, Package { name, version }));
    }

    // Parse first-party dependencies.
    let (input, version) = package_version(input)?;
    let version = PackageVersion::FirstParty(version.trim().into());

    // Ensure line is empty after the dependency.
    line_done(input)?;

    Ok((input, Package { name, version }))
}

/// Recognize local package overrides like `-e /tmp/editable`.
///
/// We'll use `/tmp/editable` as name here, since there's no other identifier
/// attached. The path is left empty since this is usually just a git
/// repository, which does not have any path.
fn editable(input: &str) -> Result<&str, Package> {
    // Ensure `-e` is present and skip it.
    let (input, _) = ws(tag("-e"))(input)?;

    // Parse everything until the next whitespace.
    // let (input, name) = recognize(alt((take_until(" "), take_until("\t"),
    // take_until(eof))))(input)?;
    let (input, name) = take_till(|c: char| c.is_whitespace())(input)?;

    // Ensure line is empty after the dependency.
    line_done(input)?;

    Ok((input, Package { name: name.into(), version: PackageVersion::Path(None) }))
}

/// Find URI dependencies.
///
/// This includes path, git and internet dependencies.
fn uri_version(input: &str) -> Result<&str, &str> {
    let (uri, _) = ws(tag("@"))(input)?;
    Ok(("", uri))
}

fn package_name(input: &str) -> Result<&str, &str> {
    terminated(ws(identifier), opt(ws(package_extras)))(input)
}

fn package_version(input: &str) -> Result<&str, &str> {
    // Ensure no `*` is in the version.
    let (_, input) = verify(rest, |s: &str| !s.contains('*'))(input)?;

    // Skip exact version indicator.
    let (input, _) = tag("==")(input)?;

    // Take all valid semver character.
    recognize(many1(alt((alphanumeric1, tag(".")))))(input)
}

fn identifier(input: &str) -> Result<&str, &str> {
    recognize(pair(alphanumeric1, many0(alt((alphanumeric1, alt((tag("-"), tag("_"), tag("."))))))))(
        input,
    )
}

fn package_extras(input: &str) -> Result<&str, &str> {
    delimited(char('['), identifier_list, char(']'))(input)
}

fn identifier_list(input: &str) -> Result<&str, &str> {
    recognize(separated_list0(char(','), ws(identifier)))(input)
}

fn line_done(input: &str) -> Result<&str, &str> {
    let (input, _) = space0(input)?;
    eof(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also
/// consumes both leading and trailing whitespace, returning the output of
/// `inner`.
fn ws<'a, F>(inner: F) -> impl FnMut(&'a str) -> Result<&str, &str>
where
    F: Fn(&'a str) -> Result<&str, &str>,
{
    delimited(space0, inner, space0)
}
