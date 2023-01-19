use std::result::Result as StdResult;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{line_ending, not_line_ending, satisfy, space0};
use nom::combinator::recognize;
use nom::error::{VerboseError, VerboseErrorKind};
use nom::multi::{many1, many_till};
use nom::sequence::{delimited, tuple};
use nom::Err as NomErr;

use crate::parsers::{take_till_blank_line, Result};
use crate::{Package, PackageVersion, ThirdPartyVersion};

/// URL of the first-party ruby registry.
const DEFAULT_REGISTRY: &str = "https://rubygems.org/";

/// Legal non-alphanumeric characters in loose version specifications.
const LOOSE_VERSION_CHARS: &[char] = &[' ', ',', '<', '>', '=', '~', '!', '.', '-', '+'];

/// Legal non-alphanumeric characters in strict version specifications.
const STRICT_VERSION_CHARS: &[char] = &['.', '-', '+'];

#[derive(Debug)]
struct Section<'a> {
    section_type: SectionType,
    content: &'a str,
}

impl<'a> Section<'a> {
    /// Parse lockfile into dependency sections.
    fn from_lockfile(mut input: &'a str) -> Result<&str, Vec<Self>> {
        let mut sections = Vec::new();

        while !input.is_empty() {
            // Find next section head.
            let (new_input, consumed) = recognize(many_till(
                take_till_line_end,
                alt((tag("GEM"), tag("GIT"), tag("PATH"), tag("BUNDLED WITH"))),
            ))(input)?;

            // Check for type of section head.
            let section_type = if consumed.ends_with("GEM") {
                SectionType::Gem
            } else if consumed.ends_with("GIT") {
                SectionType::Git
            } else if consumed.ends_with("PATH") {
                SectionType::Path
            } else if consumed.ends_with("BUNDLED WITH") {
                break;
            } else {
                // Unreachable since our parser fails if none of the headers are found.
                unreachable!();
            };

            // Find end of section.
            let (new_input, section) = take_till_blank_line(new_input)?;

            // Add to detected dependency sections.
            sections.push(Self { section_type, content: section.trim_start() });

            input = new_input;
        }

        Ok((input, sections))
    }

    /// Get dependencies in this dependency section.
    fn packages(&self) -> Result<&'a str, Vec<Package>> {
        match self.section_type {
            SectionType::Gem => self.gem_packages(),
            SectionType::Git => self.git_packages(),
            SectionType::Path => self.path_packages(),
        }
    }

    /// Get all dependencies for a GEM section.
    fn gem_packages(&self) -> Result<&'a str, Vec<Package>> {
        let (input, remote) = remote(self.content).unwrap_or((self.content, DEFAULT_REGISTRY));

        let (input, _) = specs(input)?;

        let pkgs = input
            .lines()
            .filter_map(|line| package(line).transpose())
            .map(|pkg| {
                let SpecsPackage { name, version } = pkg?;

                let version = if remote == DEFAULT_REGISTRY {
                    PackageVersion::FirstParty(version)
                } else {
                    PackageVersion::ThirdParty(ThirdPartyVersion {
                        registry: remote.into(),
                        version,
                    })
                };

                Ok(Package { name, version })
            })
            .collect::<StdResult<_, _>>()?;

        Ok((input, pkgs))
    }

    /// Get all dependencies for a GIT section.
    fn git_packages(&self) -> Result<&'a str, Vec<Package>> {
        // Parse git keys.
        let (input, remote) = remote(self.content)?;
        let (input, revision) = revision(input)?;
        let version_uri = format!("{remote}#{revision}");

        // Parse specs section.
        let (input, _) = specs(input)?;
        let mut specs_packages: Vec<SpecsPackage> = input
            .lines()
            .filter_map(|line| package(line).transpose())
            .collect::<StdResult<_, _>>()?;

        // Bail if there isn't exactly one member in the `specs` section.
        if specs_packages.len() != 1 {
            let kind =
                VerboseErrorKind::Context("Invalid number of packages listed in git dependency");
            let error = VerboseError { errors: vec![(input, kind)] };
            return Err(NomErr::Failure(error));
        }

        let package = Package {
            name: specs_packages.remove(0).name,
            version: PackageVersion::Git(version_uri),
        };

        Ok((input, vec![package]))
    }

    /// Get all dependencies for a PATH section.
    fn path_packages(&self) -> Result<&'a str, Vec<Package>> {
        // Find filesystem path.
        let (input, path) = remote(self.content)?;

        // Parse specs section.
        let (input, _) = specs(input)?;
        let mut specs_packages: Vec<SpecsPackage> = input
            .lines()
            .filter_map(|line| package(line).transpose())
            .collect::<StdResult<_, _>>()?;

        // Bail if there isn't exactly one member in the `specs` section.
        if specs_packages.len() != 1 {
            let kind =
                VerboseErrorKind::Context("Invalid number of packages listed in path dependency");
            let error = VerboseError { errors: vec![(input, kind)] };
            return Err(NomErr::Failure(error));
        }

        let package = Package {
            name: specs_packages.remove(0).name,
            version: PackageVersion::Path(Some(path.into())),
        };

        Ok((input, vec![package]))
    }
}

/// Possible dependency sections headers.
#[derive(Debug)]
enum SectionType {
    Gem,
    Git,
    Path,
}

/// Dependency listed in a `specs` section.
#[derive(Debug)]
struct SpecsPackage {
    name: String,
    version: String,
}

pub fn parse(input: &str) -> Result<&str, Vec<Package>> {
    let (input, sections) = Section::from_lockfile(input)?;

    let mut packages = Vec::new();
    for section in sections {
        packages.append(&mut section.packages()?.1);
    }

    Ok((input, packages))
}

fn remote(input: &str) -> Result<&str, &str> {
    key(input, "remote")
}

fn revision(input: &str) -> Result<&str, &str> {
    key(input, "revision")
}

fn specs(input: &str) -> Result<&str, &str> {
    recognize(many_till(take_till_line_end, recognize(tuple((space0, tag("specs:"), line_ending)))))(
        input,
    )
}

fn package(input: &str) -> StdResult<Option<SpecsPackage>, NomErr<VerboseError<&str>>> {
    let (input, name) = package_name(input)?;
    let (_, version) = loose_package_version(input)?;

    // Skip loose dependencies.
    //
    // NOTE: Loose dependencies in the Gemfile parser are not an indication that
    // this is not a proper lockfile. The lockfile specifies the loose dependencies
    // for each strict dependency beneath the strict requirement. Each of these
    // loose dependencies is also separately listed as strict dependency with
    // all its loose dependencies.
    let version = match strict_package_version(version) {
        Ok((_, version)) => version,
        Err(_) => return Ok(None),
    };

    Ok(Some(SpecsPackage { name: name.to_string(), version: version.into() }))
}

fn package_name(input: &str) -> Result<&str, &str> {
    let (input, _) = recognize(space0)(input)?;
    recognize(alt((take_until(" "), not_line_ending)))(input)
}

/// Parser allowing for loose `(>= 1.2.0, < 2.0, != 1.2.3)` and strict
/// `(1.2.3-alpha+build3)` versions.
fn loose_package_version(input: &str) -> Result<&str, &str> {
    // Versions can be completely omitted for sub-dependencies.
    if input.is_empty() {
        return Ok(("", ""));
    }

    let (input, _) = space0(input)?;
    delimited(
        tag("("),
        recognize(many1(satisfy(|c: char| {
            c.is_ascii_alphanumeric() || LOOSE_VERSION_CHARS.contains(&c)
        }))),
        tag(")"),
    )(input)
}

/// Parser allowing only strict `1.2.3-alpha+build3` versions.
fn strict_package_version(input: &str) -> Result<&str, &str> {
    let (input, _) = space0(input)?;
    recognize(many1(satisfy(|c: char| {
        c.is_ascii_alphanumeric() || STRICT_VERSION_CHARS.contains(&c)
    })))(input)
}

/// Get the value for a key in a `   key: value` line.
fn key<'a>(input: &'a str, key: &str) -> Result<&'a str, &'a str> {
    let (input, _key) = recognize(tuple((space0, tag(key), tag(": "))))(input)?;
    take_till_line_end(input)
}

/// Take everything until a line end, swallowing the line end character
/// completely.
fn take_till_line_end(input: &str) -> Result<&str, &str> {
    let (input, consumed) = recognize(alt((take_until("\n"), take_until("\r\n"))))(input)?;
    let (input, _) = alt((tag("\n"), tag("\r\n")))(input)?;
    Ok((input, consumed))
}
