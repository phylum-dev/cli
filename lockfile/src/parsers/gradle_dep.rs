use std::result::Result as StdResult;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till};
use nom::combinator::eof;
use nom::error::VerboseError;
use phylum_types::types::package::PackageType;

use crate::parsers::Result;
use crate::{Package, PackageVersion};

pub fn parse(input: &str) -> Result<&str, Vec<Package>> {
    let mut pkgs = Vec::new();
    for line in input.lines().filter(filter_line) {
        pkgs.push(package(line)?);
    }
    Ok((input, pkgs))
}

// Filter out comments and non-package lines.
fn filter_line(line: &&str) -> bool {
    !line.starts_with('#') && !line.starts_with("empty=") && !line.trim().is_empty()
}

// Take all non-white characters until encountering whitespace or `until`.
fn not_space_until(input: &str, until: char) -> Result<&str, &str> {
    take_till(|c: char| c == until || c.is_whitespace())(input)
}

fn package(input: &str) -> StdResult<Package, nom::Err<VerboseError<&str>>> {
    let (input, group_id) = not_space_until(input, ':')?;
    let (input, _) = tag(":")(input)?;

    let (input, artifact_id) = not_space_until(input, ':')?;
    let (input, _) = tag(":")(input)?;

    let (input, version) = not_space_until(input, '=')?;
    let _ = alt((tag("="), eof))(input)?;

    Ok(Package {
        name: format!("{group_id}:{artifact_id}"),
        version: PackageVersion::FirstParty(version.to_string()),
        package_type: PackageType::Maven,
    })
}
