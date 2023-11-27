//! Yarn v1 lockfile parser.

use nom::bytes::complete::take_till;
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

    let (input, packages) = many1(entry)(input)?;
    let filtered = packages.into_iter().flatten().collect();

    Ok((input, filtered))
}

fn yarn_lock_header(input: &str) -> IResult<&str, &str> {
    recognize(opt(tuple((count(take_till_line_end, 2), multispace0))))(input)
}

fn entry(input: &str) -> IResult<&str, Option<Package>> {
    let (input, capture) = recognize(many_till(
        take_till_line_end,
        recognize(tuple((space0, alt((line_ending, eof))))),
    ))(input)?;

    let (_, my_entry) = parse_entry(capture)?;
    Ok((input, my_entry))
}

fn parse_entry(input: &str) -> IResult<&str, Option<Package>> {
    let (input, (name, version)) = context("entry", tuple((entry_name, entry_version)))(input)?;

    let version = match version {
        Some(version) => version,
        None => return Ok((input, None)),
    };

    let package = Package { version, name: name.to_string(), package_type: PackageType::Npm };

    Ok((input, Some(package)))
}

fn entry_name(input: &str) -> IResult<&str, &str> {
    // Strip optional quotes.
    let (input, _) = opt(tag(r#"""#))(input)?;

    // Strip optional aliased package name.
    let (input, _) = recognize(opt(tuple((take_until("@npm:"), tag("@npm:")))))(input)?;

    // Allow for up to one leading `@` in package name (like `@angular/cli`).
    let opt_at = opt(tag("@"));

    // Consume everything until version separator as name.
    let name_parser = tuple((opt_at, take_until("@")));
    context("name", recognize(name_parser))(input)
}

fn entry_version(input: &str) -> IResult<&str, Option<PackageVersion>> {
    // Handle path dependencies.
    if input.starts_with("@./") || input.starts_with("@../") || input.starts_with("@/") {
        return path_dep(input);
    }

    // Handle git dependencies.
    if input.starts_with("@git://") {
        return git_dep(input);
    }

    // Ignore HTTP(S) dependencies.
    //
    // These could be either git or tar dependencies, so to avoid miscategorization
    // we just ignore them.
    if input.starts_with("@http://") || input.starts_with("@https://") {
        return Ok((input, None));
    }

    // Parse version field.
    let (input, _) = take_until(r#"version"#)(input)?;
    let version_key = tuple((tag(r#"version"#), opt(tag(r#"""#)), tag(r#" ""#)));
    let version_parser = delimited(version_key, is_version, tag(r#"""#));
    let (input, version) = context("version", version_parser)(input)?;

    let package_version = PackageVersion::FirstParty(version.to_string());

    Ok((input, Some(package_version)))
}

fn path_dep(input: &str) -> IResult<&str, Option<PackageVersion>> {
    let (input, _) = tag("@")(input)?;
    let (input, path) = take_till(|c| matches!(c, '"' | ',' | ':'))(input)?;
    let package_version = PackageVersion::Path(Some(path.into()));
    Ok((input, Some(package_version)))
}

fn git_dep(input: &str) -> IResult<&str, Option<PackageVersion>> {
    // Parse resolved field.
    let (input, _) = take_until(r#"resolved"#)(input)?;
    let (input, _) = tuple((tag(r#"resolved"#), opt(tag(r#"""#)), tag(r#" ""#)))(input)?;
    let (input, url) = take_until("\"")(input)?;

    let package_version = PackageVersion::Git(url.into());

    Ok((input, Some(package_version)))
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
