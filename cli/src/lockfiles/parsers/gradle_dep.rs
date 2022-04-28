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
