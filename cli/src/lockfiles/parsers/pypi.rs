use nom::character::complete::char;

use super::*;

pub fn parse(input: &str) -> Result<&str, Vec<PackageDescriptor>> {
    let pkgs = input.lines().filter_map(package).collect::<Vec<_>>();
    Ok((input, pkgs))
}

fn filter_package_name(input: &str) -> Result<&str, &str> {
    recognize(pair(
        alphanumeric1,
        many0(alt((alphanumeric1, alt((tag("-"), tag("_"), tag(".")))))),
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

fn filter_pip_name(input: &str) -> Result<&str, &str> {
    let (_, input) = verify(rest, |s: &str| {
        s.contains('@') && (s.contains("http://") || s.contains("https://"))
    })(input)?;
    recognize(alt((take_until("@"), not_line_ending)))(input)
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

fn filter_line(input: &str) -> Result<&str, &str> {
    // filter out comments, features, and install options
    let (_, input) = verify(rest, |s: &str| !s.starts_with('#'))(input)?;
    recognize(alt((take_until(";"), take_until("--"), not_line_ending)))(input)
}

fn package(input: &str) -> Option<PackageDescriptor> {
    let (_, name) = filter_line(input).ok()?;
    let (name, version) = match filter_git_repo(name).ok() {
        Some((_, s)) => match filter_egg_name(s).ok() {
            Some((n, v)) => (
                n.trim_start_matches("#egg=").to_string(),
                v.trim_start_matches("-e").to_string(),
            ),
            None => {
                let (version, name) = filter_pip_name(s).ok()?;
                (
                    name.to_string(),
                    version.trim_start_matches('@').to_string(),
                )
            }
        },
        None => {
            let (version, name) = filter_package_name(name).ok()?;
            let name: String = name.to_string().split_whitespace().collect();
            (name, version.to_string())
        }
    };

    let version: String = match get_package_version(version.trim()).ok() {
        Some((_, version)) => Some(version.to_string().split_whitespace().collect()),
        None => match get_git_version(&version).ok() {
            Some((_, s)) => Some(s.to_string()),
            None => {
                log::warn!("Could not determine version for package: {}", name);
                None
            }
        },
    }?;

    Some(PackageDescriptor {
        name: name.trim().to_lowercase(),
        version: version.trim().to_string(),
        package_type: PackageType::PyPi,
    })
}
