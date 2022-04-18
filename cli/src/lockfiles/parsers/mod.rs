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

pub mod gem;
pub mod gradle_dep;
pub mod pypi;
pub mod yarn;

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
