//! Yarn v1 lockfile parser.

use nom::bytes::complete::take_till;
use nom::multi::many0;
use nom::{Input, Parser};
use phylum_types::types::package::PackageType;

use super::*;
use crate::{Package, PackageVersion};

pub fn parse(mut input: &str) -> IResult<&str, Vec<Package>> {
    let mut packages = Vec::new();
    while !input.trim().is_empty() {
        let lockfile_entry = entry(input)?;
        if let Some(package) = lockfile_entry.1 {
            packages.push(package);
        }
        input = lockfile_entry.0;
    }
    Ok(("", packages))
}

fn entry(input: &str) -> IResult<&str, Option<Package>> {
    // Ignore comments.
    if let Ok((input, _)) = recognize((tag("#"), take_till_line_end)).parse(input) {
        let (input, _) = many0(line_ending).parse(input)?;
        return Ok((input, None));
    }

    let (input, capture) =
        recognize(many_till(take_till_line_end, recognize((space0, alt((line_ending, eof))))))
            .parse(input)?;

    let (_, my_entry) = parse_entry(capture)?;
    Ok((input, my_entry))
}

fn parse_entry(input: &str) -> IResult<&str, Option<Package>> {
    let (input, (name, version)) = context("entry", (entry_name, entry_version)).parse(input)?;

    let version = match version {
        Some(version) => version,
        None => return Ok((input, None)),
    };

    let package = Package { version, name: name.to_string(), package_type: PackageType::Npm };

    Ok((input, Some(package)))
}

fn entry_name(input: &str) -> IResult<&str, &str> {
    // Strip optional quotes.
    let (input, _) = opt(tag(r#"""#)).parse(input)?;

    // Strip optional aliased package name.
    let (input, _) = recognize(opt((take_until("@npm:"), tag("@npm:")))).parse(input)?;

    // Allow for up to one leading `@` in package name (like `@angular/cli`).
    let opt_at = opt(tag("@"));

    // Consume everything until version separator as name.
    let name_parser = (opt_at, take_until("@"));
    context("name", recognize(name_parser)).parse(input)
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
    let version_key = (tag(r#"version"#), opt(tag(r#"""#)), tag(r#" ""#));
    let version_parser = delimited(version_key, is_version, tag(r#"""#));
    let (input, version) = context("version", version_parser).parse(input)?;

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
    let (input, _) = (tag(r#"resolved"#), opt(tag(r#"""#)), tag(r#" ""#)).parse(input)?;
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
