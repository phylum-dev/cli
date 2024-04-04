use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until, take_while};
use nom::character::complete::{line_ending, multispace0, not_line_ending, space0};
use nom::combinator::{eof, map_opt, opt, recognize};
use nom::error::{context, VerboseError, VerboseErrorKind};
use nom::multi::{many0, many1, many_till};
use nom::sequence::{delimited, preceded, tuple};
use nom::Err as NomErr;

use crate::parsers::{take_till_blank_line, take_till_line_end, IResult};
use crate::spdx::{ExternalRefs, PackageInformation, ReferenceCategory, Relationship, SpdxInfo};

pub(crate) fn parse(input: &str) -> IResult<&str, SpdxInfo> {
    let (_, relationships) = parse_relationships(input)?;
    let (_, document_describes) = parse_document_describes(input)?;
    let (i, spdx_id) = parse_spdx_id(input)?;
    let (i, packages) = many1(package)(i)?;

    Ok((i, SpdxInfo { spdx_id: spdx_id.into(), document_describes, packages, relationships }))
}

fn parse_spdx_id(input: &str) -> IResult<&str, &str> {
    let (i, _) = skip_until_tag(input, "SPDXID:")?;
    let (i, spdx_id) = take_till_line_end(i)?;
    Ok((i, spdx_id.trim()))
}

fn parse_document_describes(input: &str) -> IResult<&str, Vec<String>> {
    let (i, describes) = opt(preceded(
        take_until("DocumentDescribes:"),
        take_till(|c| c == '\n' || c == '\r'),
    ))(input)?;

    let describes_list = if let Some(describes_str) = describes {
        describes_str
            .trim_start_matches("DocumentDescribes:")
            .trim()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    } else {
        Vec::new()
    };

    Ok((i, describes_list))
}

fn skip_until_tag<'a>(input: &'a str, line_tag: &'a str) -> IResult<&'a str, ()> {
    let (i, _) = take_until(line_tag)(input)?;
    let (i, _) = tag(line_tag)(i)?;
    Ok((i, ()))
}

fn parse_relationships(input: &str) -> IResult<&str, Vec<Relationship>> {
    many0(parse_relationship)(input)
}

fn parse_relationship(input: &str) -> IResult<&str, Relationship> {
    let (i, _) = skip_until_tag(input, "Relationship:")?;
    let (i, rel) = recognize(ws(take_till_line_end))(i)?;

    let parts: Vec<&str> = rel.split_whitespace().collect();
    if parts.len() == 3 {
        Ok((i, Relationship {
            spdx_element_id: Some(parts[0].to_string()),
            relationship_type: Some(parts[1].to_string()),
            related_spdx_element: Some(parts[2].to_string()),
        }))
    } else {
        let kind = VerboseErrorKind::Context("Invalid relationship format");
        let error = VerboseError { errors: vec![(input, kind)] };
        Err(NomErr::Failure(error))
    }
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

    // PackageName is required.
    let (i, _) = tag("PackageName:")(i)?;

    let (i, name) = recognize(ws(take_till_line_end))(i)?;

    // SPDXID is required.
    let (i, _) = tag("SPDXID:")(i)?;
    let (tail, spdx_id) = recognize(ws(take_till_line_end))(i)?;

    // PackageVersion is optional.
    // Version can be obtained from PURL if present, so we don't return an error
    // here.
    let (i, has_version) = opt(tag("PackageVersion:"))(tail)?;
    let (i, v) = recognize(ws(take_till_line_end))(i)?;
    let version = has_version.map(|_| v.trim().to_string());

    // Update input.
    let i = match version {
        Some(_) => i,
        None => tail,
    };

    // PackageDownloadLocation is required.
    let (i, _) = skip_until_tag(i, "PackageDownloadLocation:")?;
    let (i, download_location) = recognize(ws(take_till_line_end))(i)?;

    // Look for external references.
    let (i, next_input) = extern_ref(i)?;
    let (_, external_ref) = opt(recognize(ws(take_till_line_end)))(i)?;

    // Package name.
    let name = name.trim();

    if let Some(external_ref) = external_ref {
        let (_, external_ref) = parse_external_refs(external_ref)?;

        Ok((next_input, PackageInformation {
            name: name.into(),
            spdx_id: spdx_id.trim().into(),
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
