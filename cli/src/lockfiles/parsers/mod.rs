use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_till, take_until};
use nom::character::complete::{line_ending, multispace0, none_of, space0};
use nom::combinator::{eof, opt, recognize};
use nom::error::{context, ParseError, VerboseError};
use nom::multi::{count, many1, many_till};
use nom::sequence::{delimited, tuple};
use nom::{AsChar, IResult};
use phylum_types::types::package::{PackageDescriptor, PackageType};

pub mod gem;
pub mod gradle_dep;
pub mod pypi;
pub mod yarn;

fn take_till_line_end(input: &str) -> Result<&str, &str> {
    recognize(tuple((alt((take_until("\n"), take_until("\r\n"))), take(1usize))))(input)
}

fn take_till_blank_line(input: &str) -> Result<&str, &str> {
    recognize(alt((take_until("\n\n"), take_until("\r\n\r\n"))))(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also
/// consumes both leading and trailing whitespace, returning the output of
/// `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(space0, inner, space0)
}

type Result<T, U> = IResult<T, U, VerboseError<T>>;
