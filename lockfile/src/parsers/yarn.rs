use nom::InputTakeAtPosition;
use phylum_types::types::package::PackageType;

use super::*;
use crate::{Package, PackageVersion};

pub fn parse(input: &str) -> IResult<&str, Vec<Package>> {
    let (input, _) = yarn_lock_header(input)?;

    // Ignore empty lockfiles.
    if input.trim().is_empty() {
        return Ok(("", Vec::new()));
    }

    many1(entry)(input)
}

fn yarn_lock_header(input: &str) -> IResult<&str, &str> {
    recognize(opt(tuple((count(take_till_line_end, 2), multispace0))))(input)
}

fn entry(input: &str) -> IResult<&str, Package> {
    let (i, capture) = recognize(many_till(
        take_till_line_end,
        recognize(tuple((space0, alt((line_ending, eof))))),
    ))(input)?;

    let (_, my_entry) = parse_entry(capture)?;
    Ok((i, my_entry))
}

fn parse_entry(input: &str) -> IResult<&str, Package> {
    context("entry", tuple((entry_name, entry_version)))(input).map(|(next_input, res)| {
        let (name, version) = res;
        (next_input, Package {
            name: name.to_string(),
            version: PackageVersion::FirstParty(version.to_string()),
            package_type: PackageType::Npm,
        })
    })
}

fn entry_name(input: &str) -> IResult<&str, &str> {
    let (i, _) = opt(tag(r#"""#))(input)?;
    let opt_at = opt(tag("@"));
    let name = tuple((opt_at, take_until("@")));
    context("name", recognize(name))(i)
}

fn entry_version(input: &str) -> IResult<&str, &str> {
    let (i, _) = take_until(r#"version"#)(input)?;
    let version_key = tuple((tag(r#"version"#), opt(tag(r#"""#)), tag(r#" ""#)));
    context("version", delimited(version_key, is_version, tag(r#"""#)))(i)
}

fn is_version(input: &str) -> IResult<&str, &str> {
    input.split_at_position1_complete(
        |item| {
            let c: char = item.as_char();
            !(c == '.' || c == '-' || c.is_alphanum())
        },
        nom::error::ErrorKind::AlphaNumeric,
    )
}
