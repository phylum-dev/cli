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
