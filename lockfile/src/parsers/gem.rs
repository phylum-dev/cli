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
