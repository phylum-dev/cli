use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{line_ending, none_of, space0};
use nom::combinator::recognize;
use nom::error::{VerboseError, VerboseErrorKind};
use nom::multi::{many1, many_till};
use nom::sequence::{delimited, tuple};
use nom::Err as NomErr;

use crate::parsers::{take_till_blank_line, Result};
use crate::{Package, PackageVersion, ThirdPartyVersion};

const DEFAULT_REGISTRY: &str = "https://rubygems.org/";

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
            let header_result = recognize(many_till(
                take_till_line_end,
                alt((tag("GEM"), tag("GIT"), tag("PATH"))),
            ))(input);

            // Stop if no more headers can be found.
            let (new_input, consumed) = match header_result {
                Ok(header_result) => header_result,
                Err(_) => break,
            };

            // Check for type of section head.
            let section_type = if consumed.ends_with("GEM") {
                SectionType::Gem
            } else if consumed.ends_with("GIT") {
                SectionType::Git
            } else if consumed.ends_with("PATH") {
                SectionType::Path
            } else {
                break;
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
            .filter_map(|line| {
                let SpecsPackage { name, version } = package(line)?;

                let version = if remote == DEFAULT_REGISTRY {
                    PackageVersion::FirstParty(version)
                } else {
                    PackageVersion::ThirdParty(ThirdPartyVersion {
                        registry: remote.into(),
                        version,
                    })
                };

                Some(Package { name, version })
            })
            .collect::<Vec<_>>();

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
        let mut specs_packages = input.lines().filter_map(package).collect::<Vec<_>>();

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
        let mut specs_packages = input.lines().filter_map(package).collect::<Vec<_>>();

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

fn package(input: &str) -> Option<SpecsPackage> {
    let (input, name) = package_name(input).ok()?;
    let (_, version) = package_version(input).ok()?;
    Some(SpecsPackage { name: name.to_string(), version: version.into() })
}

fn package_name(input: &str) -> Result<&str, &str> {
    let (input, _) = recognize(space0)(input)?;
    recognize(take_until(" "))(input)
}

fn package_version(input: &str) -> Result<&str, &str> {
    let (input, _) = space0(input)?;
    delimited(tag("("), recognize(many1(none_of(" \t()"))), tag(")"))(input)
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

#[test]
fn test() {
    let input = "Test\ning";
    let (input, consumed) = take_till_line_end(input).unwrap();
    assert_eq!(consumed, "Test");
    assert_eq!(input, "ing");
}
