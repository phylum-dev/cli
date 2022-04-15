use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::{
        complete::{alphanumeric1, line_ending, none_of, not_line_ending, space0},
        streaming::multispace0,
    },
    combinator::{eof, opt, recognize, rest, verify},
    error::{context, VerboseError},
    multi::{count, many0, many1, many_till},
    sequence::{delimited, pair, tuple},
    AsChar, IResult,
};

use phylum_types::types::package::{PackageDescriptor, PackageType};

fn take_till_line_end(input: &str) -> Result<&str, &str> {
    recognize(tuple((
        alt((take_until("\n"), take_until("\r\n"))),
        take(1usize),
    )))(input)
}

fn take_till_blank_line(input: &str) -> Result<&str, &str> {
    recognize(alt((take_until("\n\n"), take_until("\r\n\r\n"))))(input)
}

type Result<T, U> = IResult<T, U, VerboseError<T>>;

pub mod yarn {
    use super::*;

    pub fn parse(input: &str) -> Result<&str, Vec<PackageDescriptor>> {
        let (i, _) = yarn_lock_header(input)?;
        let (i, mut entries) = many1(entry)(i)?;

        // Attempt to parse one final entry not followed by a newline
        let res = entry_final(i);
        if let Ok((i, final_entry)) = res {
            entries.push(final_entry);
            return Ok((i, entries));
        }

        Ok((i, entries))
    }

    fn yarn_lock_header(input: &str) -> Result<&str, &str> {
        recognize(tuple((count(take_till_line_end, 2), multispace0)))(input)
    }

    fn entry_final(input: &str) -> Result<&str, PackageDescriptor> {
        let (i, capture) = recognize(many_till(take_till_line_end, eof))(input)?;
        let (_, my_entry) = parse_entry(capture)?;
        Ok((i, my_entry))
    }

    fn entry(input: &str) -> Result<&str, PackageDescriptor> {
        let (i, capture) = recognize(many_till(
            take_till_line_end,
            recognize(tuple((space0, line_ending))),
        ))(input)?;

        let (_, my_entry) = parse_entry(capture)?;
        Ok((i, my_entry))
    }

    fn parse_entry(input: &str) -> Result<&str, PackageDescriptor> {
        context("entry", tuple((entry_name, entry_version)))(input).map(|(next_input, res)| {
            let (name, version) = res;
            (
                next_input,
                PackageDescriptor {
                    name: name.to_string(),
                    version: version.to_string(),
                    package_type: PackageType::Npm,
                },
            )
        })
    }

    fn entry_name(input: &str) -> Result<&str, &str> {
        let (i, _) = opt(tag(r#"""#))(input)?;
        let opt_at = opt(tag("@"));
        let name = tuple((opt_at, take_until("@")));
        context("name", recognize(name))(i)
    }

    fn entry_version(input: &str) -> Result<&str, &str> {
        let (i, _) = take_until(r#"version ""#)(input)?;
        context(
            "version",
            delimited(tag(r#"version ""#), is_version, tag(r#"""#)),
        )(i)
    }

    fn is_version<T, E: nom::error::ParseError<T>>(input: T) -> IResult<T, T, E>
    where
        T: nom::InputTakeAtPosition,
        <T as nom::InputTakeAtPosition>::Item: AsChar,
    {
        input.split_at_position1_complete(
            |item| {
                let c: char = item.as_char();
                !(c == '.' || c == '-' || c.is_alphanum())
            },
            nom::error::ErrorKind::AlphaNumeric,
        )
    }
}

pub mod gem {
    use super::*;

    pub fn parse(input: &str) -> Result<&str, Vec<PackageDescriptor>> {
        let (input, _) = gem_header(input)?;
        let (i, consumed) = specs(input)?;
        let pkgs = consumed.lines().filter_map(package).collect::<Vec<_>>();
        Ok((i, pkgs))
    }

    fn gem_header(input: &str) -> Result<&str, &str> {
        let (input, _) = recognize(take_until("GEM"))(input)?;
        recognize(tuple((tag("GEM"), line_ending)))(input)
    }

    fn specs(input: &str) -> Result<&str, &str> {
        let (input, _consumed) = recognize(many_till(
            take_till_line_end,
            recognize(tuple((space0, tag("specs:"), line_ending))),
        ))(input)?;

        take_till_blank_line(input)
    }

    fn package_name(input: &str) -> Result<&str, &str> {
        let (input, _) = recognize(space0)(input)?;
        recognize(take_until(" "))(input)
    }

    fn package_version(input: &str) -> Result<&str, &str> {
        let (input, _) = space0(input)?;
        delimited(tag("("), recognize(many1(none_of(" \t()"))), tag(")"))(input)
    }

    fn package(input: &str) -> Option<PackageDescriptor> {
        let (input, name) = package_name(input).ok()?;
        let (_, version) = package_version(input).ok()?;

        Some(PackageDescriptor {
            name: name.to_string(),
            version: version.to_string(),
            package_type: PackageType::RubyGems,
        })
    }
}

pub mod pypi {
    use nom::{character::complete::char, multi::separated_list1, sequence::terminated};

    use crate::lockfiles::ParseResult;

    use super::*;

    pub fn parse(input: &str) -> ParseResult {
        let lines = input.lines().filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                None
            } else {
                Some(line)
            }
        });

        let mut pkgs = Vec::new();
        for line in lines {
            let pkg = match package(line) {
                Ok((_, pkg)) => pkg,
                Err(e) => {
                    log::debug!("nom error: {:?}", e);
                    return Err(format!("Failed to parse requirement: {}", line).into());
                }
            };
            pkgs.push(pkg);
        }
        Ok(pkgs)
    }

    fn package_identifier(input: &str) -> Result<&str, &str> {
        recognize(pair(
            alphanumeric1,
            many0(alt((
                alphanumeric1,
                recognize(pair(
                    many0(alt((tag("-"), tag("_"), tag(".")))),
                    alphanumeric1,
                )),
            ))),
        ))(input)
    }

    fn filter_git_repo(input: &str) -> Result<&str, &str> {
        let (x, input) = verify(rest, |s: &str| {
            s.contains("https://") || s.contains("http://") || s.starts_with("-e")
        })(input)?;
        Ok((x, input))
    }

    fn filter_egg_name(input: &str) -> Result<&str, &str> {
        let (_, input) = verify(rest, |s: &str| s.contains("#egg="))(input)?;
        recognize(alt((take_until("#egg="), not_line_ending)))(input)
    }

    fn get_package_version(input: &str) -> Result<&str, &str> {
        let (_, input) = verify(rest, |s: &str| !s.contains('*'))(input)?;
        delimited(
            tag("=="),
            recognize(many1(alt((alphanumeric1, recognize(char('.')), tag(" "))))),
            rest,
        )(input)
    }

    fn get_git_version(input: &str) -> Result<&str, &str> {
        verify(rest, |s: &str| {
            s.contains("http://") || s.contains("https://")
        })(input)
    }

    fn package_extras(input: &str) -> Result<&str, &str> {
        delimited(
            pair(space0, tag("[")),
            recognize(separated_list1(
                pair(space0, tag(",")),
                pair(space0, package_identifier),
            )),
            pair(space0, tag("]")),
        )(input)
    }

    fn package_name(input: &str) -> Result<&str, &str> {
        let (input, _) = space0(input)?;
        terminated(package_identifier, opt(package_extras))(input)
    }

    fn package(input: &str) -> Result<&str, PackageDescriptor> {
        let (input, name) = match filter_git_repo(input).ok() {
            Some((_, s)) => match filter_egg_name(s).ok() {
                Some((n, v)) => (
                    v.trim_start_matches("-e"),
                    n.trim_start_matches("#egg=").to_string(),
                ),
                None => {
                    let (input, name) = package_name(s)?;
                    (input.trim().trim_start_matches('@'), name.to_string())
                }
            },
            None => {
                let (input, name) = package_name(input)?;
                let name: String = name.to_string().split_whitespace().collect();
                (input, name)
            }
        };

        let input = input.trim();

        let version = match get_package_version(input).ok() {
            Some((_, version)) => version.to_string().split_whitespace().collect(),
            None => {
                let (_, v) = get_git_version(input)?;
                v.to_string()
            }
        };

        Ok((
            "",
            PackageDescriptor {
                name: name.trim().to_lowercase(),
                version: version.trim().to_string(),
                package_type: PackageType::PyPi,
            },
        ))
    }
}

pub mod gradle_dep {
    use nom::sequence::preceded;

    use super::*;

    pub fn parse(input: &str) -> Result<&str, Vec<PackageDescriptor>> {
        let pkgs = input.lines().filter_map(package).collect::<Vec<_>>();
        Ok((input, pkgs))
    }

    fn group_id(input: &str) -> Result<&str, &str> {
        recognize(take_until(":"))(input)
    }

    fn artifact_id_version(input: &str) -> Result<&str, &str> {
        let (input, artifact_id) = delimited(tag(":"), take_until(":"), tag(":"))(input)?;
        let (_, version) = recognize(alt((take_until(" ("), not_line_ending)))(input)?;
        Ok((artifact_id, version))
    }

    fn filter_line(input: &str) -> Result<&str, &str> {
        let (input, _) = recognize(alt((
            take_until("+---"),
            take_until("\\---"),
            not_line_ending,
        )))(input)?;
        preceded(alt((tag("+--- "), tag("\\--- "))), not_line_ending)(input)
    }

    fn package(input: &str) -> Option<PackageDescriptor> {
        let (_, input) = filter_line(input).ok()?;
        let (input, group_id) = group_id(input).ok()?;
        let (artifact_id, version) = artifact_id_version(input).ok()?;

        Some(PackageDescriptor {
            name: format!("{}:{}", group_id, artifact_id),
            version: version.to_string(),
            package_type: PackageType::Maven,
        })
    }
}
