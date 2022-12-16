use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_until};
use nom::character::complete::{line_ending, multispace0, space0};
use nom::combinator::{eof, opt, recognize};
use nom::error::{context, VerboseError};
use nom::multi::{count, many1, many_till};
use nom::sequence::{delimited, tuple};
use nom::{AsChar, IResult};

pub mod gem;
pub mod go_sum;
pub mod gradle_dep;
pub mod pypi;
pub mod yarn;

fn take_till_line_end(input: &str) -> Result<&str, &str> {
    recognize(tuple((alt((take_until("\n"), take_until("\r\n"))), take(1usize))))(input)
}

fn take_till_blank_line(input: &str) -> Result<&str, &str> {
    recognize(alt((take_until("\n\n"), take_until("\r\n\r\n"))))(input)
}

type Result<T, U> = IResult<T, U, VerboseError<T>>;
