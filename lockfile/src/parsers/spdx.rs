use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while};
use nom::character::complete::{multispace0, not_line_ending, space0};
use nom::character::streaming::line_ending;
use nom::combinator::{eof, map_opt, opt, recognize};
use nom::error::context;
use nom::multi::{many1, many_till};
use nom::sequence::{delimited, tuple};

use crate::parsers::{take_till_blank_line, take_till_line_end, Result};
use crate::spdx::{ExternalRefs, PackageInformation, ReferenceCategory};
use crate::Package;

pub fn parse(input: &str) -> Result<&str, Vec<Package>> {
    let (input, mut pkg_options) = many1(package)(input)?;
    let pkgs = pkg_options
        .drain(..)
        .filter_map(|pi| Package::try_from(&pi).ok())
        .collect::<Vec<Package>>();

    Ok((input, pkgs))
}

fn package_name(input: &str) -> Result<&str, &str> {
    recognize(take_until("PackageName:"))(input)
}

fn package(input: &str) -> Result<&str, PackageInformation> {
    let (i, _) = package_name(input)?;

    let (i, capture) = recognize(many_till(
        take_till_line_end,
        recognize(tuple((space0, alt((line_ending, eof))))),
    ))(i)?;

    let (i, _) = alt((package_name, line_ending, eof))(i)?;

    let (_, my_entry) = parse_package(capture)?;
    Ok((i, my_entry))
}

fn parse_package(input: &str) -> Result<&str, PackageInformation> {
    context("package", package_info)(input).map(|(next_input, res)| {
        let pi = res;
        (next_input, pi)
    })
}

fn package_info(input: &str) -> Result<&str, PackageInformation> {
    let (i, _) = package_name(input)?;

    // PackageName is required
    let (i, _) = tag("PackageName:")(i)?;
    let (i, name) = recognize(ws(take_till_line_end))(i)?;

    // SPDXID is required
    let (i, _) = tag("SPDXID:")(i)?;
    let (tail, _) = recognize(ws(take_till_line_end))(i)?;

    // PackageVersion is optional
    // Version can be obtained from PURL if present, so we don't return an error
    // here
    let (i, has_version) = opt(tag("PackageVersion:"))(tail)?;
    let (i, v) = recognize(ws(take_till_line_end))(i)?;
    let version = has_version.map(|_| v.trim().to_string());

    // Update input
    let i = match version {
        Some(_) => i,
        None => tail,
    };

    // PackageDownloadLocation is required
    let (i, _) = tag("PackageDownloadLocation:")(i)?;
    let (i, _) = recognize(ws(take_till_line_end))(i)?;

    // Look for external references
    let (i, next_input) = extern_ref(i)?;
    let (_, external_ref) = opt(recognize(ws(take_till_line_end)))(i)?;

    if let Some(external_ref) = external_ref {
        let (_, external_ref) = parse_external_refs(external_ref)?;

        Ok((next_input, PackageInformation {
            name: name.trim().into(),
            version_info: version,
            download_location: "NOASSERTION".into(),
            external_refs: vec![external_ref],
        }))
    } else {
        Ok((next_input, PackageInformation::default()))
    }
}

fn extern_ref(input: &str) -> Result<&str, &str> {
    recognize(alt((
        take_until("ExternalRef: PACKAGE-MANAGER purl"),
        take_until("ExternalRef: PACKAGE_MANAGER purl"),
        take_till_blank_line,
    )))(input)
}

fn parse_external_refs(input: &str) -> Result<&str, ExternalRefs> {
    let input = input.trim_start_matches("ExternalRef:").trim();
    let purl = tuple((
        ws(take_while(|c: char| !c.is_whitespace())),
        ws(take_while(|c: char| !c.is_whitespace())),
        ws(not_line_ending),
    ));

    map_opt(purl, |(reference_category, reference_type, reference_locator)| {
        let reference_category = match reference_category {
            "SECURITY" => Some(ReferenceCategory::Security),
            "PACKAGE-MANAGER" | "PACKAGE_MANAGER" => Some(ReferenceCategory::PackageManager),
            "PERSISTENT-ID" | "PERSISTENT_ID" => Some(ReferenceCategory::PersistentId),
            "OTHER" => Some(ReferenceCategory::Other),
            _ => None,
        }?;

        Some(ExternalRefs {
            reference_category,
            reference_type: reference_type.into(),
            reference_locator: reference_locator.into(),
        })
    })(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also
/// consumes both leading and trailing whitespace, returning the output of
/// `inner`.
fn ws<'a, F>(inner: F) -> impl FnMut(&'a str) -> Result<&str, &str>
where
    F: Fn(&'a str) -> Result<&str, &str>,
{
    delimited(multispace0, inner, multispace0)
}
