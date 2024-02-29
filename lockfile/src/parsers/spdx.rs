use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while};
use nom::character::complete::{line_ending, multispace0, not_line_ending, space0};
use nom::combinator::{eof, map_opt, opt, recognize};
use nom::error::{context, VerboseError, VerboseErrorKind};
use nom::multi::{many1, many_till};
use nom::sequence::{delimited, tuple};
use nom::Err as NomErr;

use crate::parsers::{take_till_blank_line, take_till_line_end, IResult};
use crate::spdx::{ExternalRefs, PackageInformation, ReferenceCategory};

pub(crate) fn parse(input: &str) -> IResult<&str, Vec<PackageInformation>> {
    let (i, pkgs_info) = many1(package)(input)?;
    Ok((i, pkgs_info))
}

fn package_name(input: &str) -> IResult<&str, &str> {
    recognize(take_until("PackageName:"))(input)
}

fn package(input: &str) -> IResult<&str, PackageInformation> {
    let (i, _) = package_name(input)?;

    let (i, capture) = recognize(many_till(
        take_till_line_end,
        recognize(tuple((space0, alt((line_ending, eof))))),
    ))(i)?;

    let (_, my_entry) = parse_package(capture)?;
    Ok((i, my_entry))
}

fn parse_package(input: &str) -> IResult<&str, PackageInformation> {
    context("package", package_info)(input).map(|(next_input, res)| {
        let pi = res;
        (next_input, pi)
    })
}

fn package_info(input: &str) -> IResult<&str, PackageInformation> {
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
    let (i, download_location) = recognize(ws(take_till_line_end))(i)?;

    // Look for external references
    let (i, next_input) = extern_ref(i)?;
    let (_, external_ref) = opt(recognize(ws(take_till_line_end)))(i)?;

    // Package name
    let name = name.trim();

    if let Some(external_ref) = external_ref {
        let (_, external_ref) = parse_external_refs(external_ref)?;

        Ok((next_input, PackageInformation {
            name: name.into(),
            version_info: version,
            download_location: download_location.into(),
            external_refs: vec![external_ref],
        }))
    } else {
        let kind = VerboseErrorKind::Context("Missing package locator");
        let error = VerboseError { errors: vec![(input, kind)] };
        Err(NomErr::Failure(error))
    }
}

fn extern_ref(input: &str) -> IResult<&str, &str> {
    recognize(alt((
        take_until("ExternalRef: PACKAGE-MANAGER"),
        take_until("ExternalRef: PACKAGE_MANAGER"),
        take_till_blank_line,
    )))(input)
}

fn parse_external_refs(input: &str) -> IResult<&str, ExternalRefs> {
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
fn ws<'a, F>(inner: F) -> impl FnMut(&'a str) -> IResult<&str, &str>
where
    F: Fn(&'a str) -> IResult<&str, &str>,
{
    delimited(multispace0, inner, multispace0)
}
