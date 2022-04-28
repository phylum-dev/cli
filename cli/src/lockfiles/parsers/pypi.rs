use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alphanumeric1, char, not_line_ending},
    combinator::{opt, recognize, rest, verify},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, pair, terminated},
};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::{ws, Result};

pub fn parse(input: &str) -> Result<&str, Vec<PackageDescriptor>> {
    let pkgs = input.lines().filter_map(package).collect::<Vec<_>>();
    Ok((input, pkgs))
}

fn filter_package_name(input: &str) -> Result<&str, &str> {
    terminated(ws(identifier), opt(ws(package_extras)))(input)
}

fn identifier(input: &str) -> Result<&str, &str> {
    recognize(pair(
        alphanumeric1,
        many0(alt((alphanumeric1, alt((tag("-"), tag("_"), tag(".")))))),
    ))(input)
}

fn identifier_list(input: &str) -> Result<&str, &str> {
    recognize(separated_list0(char(','), ws(identifier)))(input)
}

fn package_extras(input: &str) -> Result<&str, &str> {
    delimited(char('['), identifier_list, char(']'))(input)
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
                let (version, pkg) = filter_pip_name(s).ok()?;
                let (_, name) = filter_package_name(pkg).ok()?;
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn package_with_extras() {
        assert_eq!(
            package("celery [ redis ] == 5.0.5"),
            Some(PackageDescriptor {
                name: "celery".into(),
                version: "5.0.5".into(),
                package_type: PackageType::PyPi,
            })
        );

        assert_eq!(
            package("requests[security,socks]==2.27.1"),
            Some(PackageDescriptor {
                name: "requests".into(),
                version: "2.27.1".into(),
                package_type: PackageType::PyPi,
            })
        );

        assert_eq!(
            package("git-for-pip-example[PDF] @ git+https://github.com/matiascodesal/git-for-pip-example.git@v1.0.0"),
            Some(PackageDescriptor {
                name: "git-for-pip-example".into(),
                version: "git+https://github.com/matiascodesal/git-for-pip-example.git@v1.0.0".into(),
                package_type: PackageType::PyPi,
            })
        );
    }
}
