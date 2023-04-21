use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::character::complete::{alphanumeric1, char, not_line_ending, space0};
use nom::combinator::{eof, opt, recognize, rest, verify};
use nom::error::{VerboseError, VerboseErrorKind};
use nom::multi::{many0, many1, separated_list0};
use nom::sequence::{delimited, pair, terminated};
use nom::Err as NomErr;
use phylum_types::types::package::PackageType;

use crate::parsers::Result;
use crate::{Package, PackageVersion};

pub fn parse(input: &str) -> Result<&str, Vec<Package>> {
    let mut pkgs = Vec::new();

    // Iterate over all non-empty lines.
    for line in
        input.lines().filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
    {
        let (_, line) = recognize(alt((take_until(" #"), not_line_ending)))(line)?;
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
            PackageVersion::DownloadUrl(uri_version.into())
        };

        return Ok((input, Package { name, version, package_type: PackageType::PyPi }));
    }

    // Parse first-party dependencies.
    let (input, version) = package_version(input)?;
    let version = PackageVersion::FirstParty(version.trim().into());

    // Ensure line is empty after the dependency.
    line_done(input)?;

    Ok((input, Package { name, version, package_type: PackageType::PyPi }))
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
    let (input, uri) = take_till(|c: char| c.is_whitespace())(input)?;

    // Detect version based on URI prefix.
    let (name, version) = if uri.starts_with("git+") {
        // Split up git URI and dependency name.
        match uri.rsplit_once("#egg=") {
            Some((uri, egg)) => {
                // Replace last `@` in URI with `#`
                // git+ssh://github.com:org/project@HASH
                //   -> git+ssh://github.com:org/project#HASH
                let uri = uri
                    .rsplit_once('@')
                    .map(|(head, tail)| format!("{head}#{tail}"))
                    .unwrap_or_else(|| uri.into());

                (egg.into(), PackageVersion::Git(uri))
            },
            None => {
                let kind = VerboseErrorKind::Context("Missing egg name in git URI");
                let error = VerboseError { errors: vec![(input, kind)] };
                return Err(NomErr::Failure(error));
            },
        }
    } else {
        // Assume non-git editable dependencies are paths.
        let path = PathBuf::from(uri);
        let name = path.file_name().unwrap_or(path.as_os_str());
        (name.to_string_lossy().into(), PackageVersion::Path(Some(path)))
    };

    // Ensure line is empty after the dependency.
    line_done(input)?;

    Ok((input, Package { name, version, package_type: PackageType::PyPi }))
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
