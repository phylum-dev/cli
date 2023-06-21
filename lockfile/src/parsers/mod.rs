use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{line_ending, multispace0, not_line_ending, space0};
use nom::combinator::{eof, opt, recognize, rest};
use nom::error::{context, VerboseError};
use nom::multi::{count, many1, many_till};
use nom::sequence::{delimited, terminated, tuple};
use nom::{AsChar, IResult};

pub mod gem;
pub mod go_sum;
pub mod gradle_dep;
pub mod pypi;
pub mod spdx;
pub mod yarn;

/// Consume everything until the next `\n` or `\r\n`.
fn take_till_line_end(input: &str) -> Result<&str, &str> {
    recognize(terminated(not_line_ending, line_ending))(input)
}

/// Consume everything until the next `\n\n` or `\r\n\r\n`.
fn take_till_blank_line(input: &str) -> Result<&str, &str> {
    recognize(alt((take_until("\n\n"), take_until("\r\n\r\n"))))(input)
}

/// Consume the next line.
///
/// This supports both `\n` and `\r\n`. It also skips line continuations
/// (`\\\n`, `\\\r\n`) and stops on EOF.
fn take_continued_line(mut input: &str) -> Result<&str, ()> {
    loop {
        // Get everything up to the next NL or EOF.
        let (new_input, line) = recognize(alt((take_till_line_end, rest)))(input)?;
        input = new_input;

        // Stop consuming lines once there are no continuations.
        if !line.ends_with("\\\n") && !line.ends_with("\\\r\n") {
            break;
        }
    }

    Ok((input, ()))
}

type Result<T, U> = IResult<T, U, VerboseError<T>>;
