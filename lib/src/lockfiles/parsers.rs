use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::{
        complete::{line_ending, none_of, space0},
        streaming::multispace0,
    },
    combinator::{eof, opt, recognize},
    error::{context, VerboseError},
    multi::{count, many0, many1, many_till},
    sequence::{delimited, tuple},
    AsChar, IResult,
};

use crate::types::{PackageDescriptor, PackageType};

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
        let (i, mut entries) = many0(entry)(i)?;
        let (i, final_entry) = entry_final(i)?;
        entries.push(final_entry);
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
                    r#type: PackageType::Npm,
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
        let pkgs = consumed
            .lines()
            .map(|l| package(l))
            .filter_map(|x| x)
            .collect::<Vec<_>>();
        Ok((i, pkgs))
    }

    fn gem_header(input: &str) -> Result<&str, &str> {
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
            r#type: PackageType::Ruby,
        })
    }
}
