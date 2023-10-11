use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::character::complete::{alphanumeric1, char, line_ending, space1};
use nom::combinator::{eof, opt, recognize, rest, verify};
use nom::error::{VerboseError, VerboseErrorKind};
use nom::multi::{many0, many1, separated_list0};
use nom::sequence::{delimited, pair, terminated};
use nom::Err as NomErr;
use phylum_types::types::package::PackageType;

use crate::parsers::{self, IResult};
use crate::{Package, PackageVersion, ThirdPartyVersion};

pub fn parse(mut input: &str) -> IResult<&str, Vec<Package>> {
    let mut pkgs = Vec::new();

    let mut registry = None;
    while !input.is_empty() {
        // Get the next line.
        let (new_input, line) = line(input, &mut registry)?;
        input = new_input;

        // Ignore empty lines.
        if line.is_empty() {
            continue;
        }

        // Strip comments.
        let (_, line) = alt((take_until(" #"), rest))(line)?;

        // Parse dependency.
        let (_, pkg) = package(line, registry)?;
        pkgs.push(pkg);
    }

    Ok((input, pkgs))
}

/// Parse one line in the lockfile.
fn line<'a>(input: &'a str, registry: &mut Option<&'a str>) -> IResult<&'a str, &'a str> {
    // Take everything until the next newline.
    //
    // This takes line continuation characters into account.
    let (input, mut line) = recognize(parsers::take_continued_line)(input)?;

    // Remove irrelevant whitespace.
    line = line.trim();

    // Remove entirely commented out lines.
    if line.starts_with('#') {
        line = "";
    }

    // Ignore index config options.
    //
    // Since `ThirdPartyVersion` only allows a single registry, we prefer recording
    // only the primary one.
    if let Some(index_url) = line.strip_prefix("--index-url") {
        *registry = Some(index_url.trim());
        line = "";
    }
    if let Some(extra_index_url) = line.strip_prefix("--extra-index-url") {
        // Only use extra index URL if no other URL has been specified.
        if registry.is_none() {
            *registry = Some(extra_index_url.trim());
        }
        line = "";
    }

    Ok((input, line))
}

fn package<'a>(input: &'a str, registry: Option<&str>) -> IResult<&'a str, Package> {
    // Ignore everything after `;`.
    let (_, input) = alt((take_until(";"), rest))(input)?;

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
    let version = match registry {
        Some(registry) => PackageVersion::ThirdParty(ThirdPartyVersion {
            version: version.trim().into(),
            registry: registry.into(),
        }),
        None => PackageVersion::FirstParty(version.trim().into()),
    };

    // Ensure line is empty after the dependency.
    line_done(input)?;

    Ok((input, Package { name, version, package_type: PackageType::PyPi }))
}

/// Recognize local package overrides like `-e /tmp/editable`.
///
/// We'll use `/tmp/editable` as name here, since there's no other identifier
/// attached. The path is left empty since this is usually just a git
/// repository, which does not have any path.
fn editable(input: &str) -> IResult<&str, Package> {
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
fn uri_version(input: &str) -> IResult<&str, &str> {
    let (uri, _) = ws(tag("@"))(input)?;
    Ok(("", uri))
}

fn package_name(input: &str) -> IResult<&str, &str> {
    terminated(ws(identifier), opt(ws(package_extras)))(input)
}

fn package_version(input: &str) -> IResult<&str, &str> {
    // Ensure no `*` is in the version.
    let (_, input) = verify(rest, |s: &str| !s.contains('*'))(input)?;

    // Skip exact version indicator.
    let (input, _) = tag("==")(input)?;

    // Take all valid semver character.
    recognize(many1(alt((alphanumeric1, tag(".")))))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(alphanumeric1, many0(alt((alphanumeric1, alt((tag("-"), tag("_"), tag("."))))))))(
        input,
    )
}

fn package_extras(input: &str) -> IResult<&str, &str> {
    delimited(char('['), identifier_list, char(']'))(input)
}

fn identifier_list(input: &str) -> IResult<&str, &str> {
    recognize(separated_list0(char(','), ws(identifier)))(input)
}

fn line_done(input: &str) -> IResult<&str, &str> {
    // Allow for spaces and arguments not impacting resolution.
    let (input, _) = recognize(many0(alt((nl_space1, package_hash))))(input)?;

    eof(input)
}

/// Parse package hashes.
///
/// Example:
///   --hash=sha256:
/// 8c2f9abd47a9e8df7f0c3f091ce9497d011dc3b31effcf4c85a6e2b50f4114ef
fn package_hash(input: &str) -> IResult<&str, &str> {
    // Argument name.
    let (input, _) = tag("--hash=")(input)?;

    // Hash variant.
    let (input, _) = alphanumeric1(input)?;

    // Separator.
    let (input, _) = tag(":")(input)?;

    // Package hash.
    alphanumeric1(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also
/// consumes both leading and trailing whitespace, returning the output of
/// `inner`.
fn ws<'a, F>(inner: F) -> impl FnMut(&'a str) -> IResult<&str, &str>
where
    F: Fn(&'a str) -> IResult<&str, &str>,
{
    delimited(nl_space0, inner, nl_space0)
}

/// Newline-aware space0.
///
/// This automatically handles " \\\n" and treats it as normal space.
fn nl_space0(input: &str) -> IResult<&str, &str> {
    recognize(many0(alt((space1, line_continuation))))(input)
}

/// Newline-aware space1.
///
/// This automatically handles " \\\n" and treats it as normal space.
fn nl_space1(input: &str) -> IResult<&str, &str> {
    recognize(many1(alt((space1, line_continuation))))(input)
}

/// Recognize line continuations.
fn line_continuation(input: &str) -> IResult<&str, &str> {
    recognize(pair(tag("\\"), line_ending))(input)
}
