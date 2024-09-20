use std::collections::HashSet;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_till1, take_while};
use nom::character::complete::{char, multispace0, space0, space1};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;

use crate::golang::GoDeps;
use crate::{Package, PackageType, PackageVersion};

#[derive(Debug, PartialEq, Eq)]
pub enum Directive<'a> {
    Module(&'a str),
    Go(&'a str),
    Require(Vec<Module>),
    Exclude(Vec<Module>),
    Replace(Vec<ModuleReplacement>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module {
    path: String,
    version: String,
    indirect: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Replacement {
    Module(Module),
    FilePath(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleReplacement {
    path: String,
    version: Option<String>,
    replacement: Replacement,
}

impl From<Module> for Package {
    fn from(module: Module) -> Self {
        Self {
            name: module.path,
            version: PackageVersion::FirstParty(module.version),
            package_type: PackageType::Golang,
        }
    }
}

impl From<ModuleReplacement> for Package {
    fn from(module_replacement: ModuleReplacement) -> Self {
        match module_replacement.replacement {
            Replacement::Module(module) => Self {
                name: module.path,
                version: PackageVersion::FirstParty(module.version),
                package_type: PackageType::Golang,
            },
            Replacement::FilePath(path) => Self {
                name: path.clone(),
                version: PackageVersion::Path(Some(path.into())),
                package_type: PackageType::Golang,
            },
        }
    }
}

pub fn parse(input: &str) -> IResult<&str, GoDeps> {
    let (_, directives) = many0(directive)(input)?;

    let mut required: Vec<Module> = Vec::new();
    let mut excluded: Vec<Module> = Vec::new();
    let mut replaced: Vec<ModuleReplacement> = Vec::new();
    let mut go_directive = String::new();
    let mut go_module = String::new();

    for directive in directives {
        match directive {
            Directive::Module(module) => module.clone_into(&mut go_module),
            Directive::Go(go) => go.clone_into(&mut go_directive),
            Directive::Require(modules) => required.extend(modules),
            Directive::Exclude(modules) => excluded.extend(modules),
            Directive::Replace(modules) => replaced.extend(modules),
        };
    }

    let mut modules: HashSet<Module> = HashSet::from_iter(required);
    let excluded_set: HashSet<Module> = HashSet::from_iter(excluded);
    let replacement_set: HashSet<ModuleReplacement> = HashSet::from_iter(replaced);

    // Remove excluded modules from required modules.
    modules.retain(|module| !excluded_set.contains(module));

    let mut packages: Vec<Package> = Vec::new();

    for replacement in replacement_set {
        let module_path = &replacement.path;

        // Check if the replacement module version is available and remove any modules
        // marked for replacement.
        match &replacement.version {
            Some(version) => {
                let module = Module {
                    path: module_path.to_owned(),
                    version: version.to_owned(),
                    indirect: false,
                };
                modules.remove(&module);
            },
            None => {
                // Remove all modules with the same path since version isn't specified.
                modules.retain(|m| (&m.path != module_path) || m.indirect);
            },
        }

        // Add the replacement module.
        packages.push(Package::from(replacement));
    }

    packages.extend(modules.into_iter().map(Package::from));

    Ok((input, GoDeps { go: go_directive, modules: packages }))
}

fn directive(input: &str) -> IResult<&str, Directive> {
    let (input, _) = take_while(|c: char| c == '\n')(input)?;
    alt((module_directive, go_directive, require_directive, replace_directive, exclude_directive))(
        input.trim(),
    )
}

fn module_directive(input: &str) -> IResult<&str, Directive> {
    let (input, module_name) =
        preceded(tuple((tag("module"), space1)), take_till(|c| c == '\n'))(input)?;
    Ok((input, Directive::Module(module_name)))
}

fn go_directive(input: &str) -> IResult<&str, Directive> {
    let (input, go_version) =
        preceded(tuple((tag("go"), space1)), take_till(|c| c == '\n'))(input)?;
    Ok((input, Directive::Go(go_version.trim())))
}

fn require_directive(input: &str) -> IResult<&str, Directive> {
    let (input, deps) = preceded(
        tuple((tag("require"), space1)),
        alt((module_block, map(require_spec, |r| vec![r]))),
    )(input)?;
    Ok((input, Directive::Require(deps)))
}

fn require_spec(input: &str) -> IResult<&str, Module> {
    let (input, module_path) = take_till1(|c: char| c.is_whitespace())(input)?;
    let (input, _) = space1(input)?;
    let (input, version) = take_till1(|c: char| c.is_whitespace() || c == '/')(input)?;
    let (input, _) = space0(input)?;

    // Check if there is a comment starting with "//".
    let (input, comments) = opt(preceded(tag("//"), take_till1(|c: char| c == '\n')))(input)?;

    // Determine if the comment indicates the module is indirect.
    let indirect = comments.map_or(false, |s: &str| s.trim().eq("indirect"));
    let (input, _) = take_while(|c: char| c != '\n')(input)?;

    Ok((input, Module { path: module_path.to_string(), version: version.to_string(), indirect }))
}

fn replace_directive(input: &str) -> IResult<&str, Directive> {
    preceded(tuple((tag("replace"), space1)), alt((replace_block, map(replace_spec, |r| vec![r]))))(
        input,
    )
    .map(|(next_input, reps)| (next_input, Directive::Replace(reps)))
}

fn replace_spec(input: &str) -> IResult<&str, ModuleReplacement> {
    let (input, src_path) = take_till1(|c: char| c.is_whitespace() || c == '=' || c == '>')(input)?;

    // Try to detect if there is a version by checking for the presence of '=>'.
    let (input, src_version) = if input.trim_start().starts_with("=>") {
        (input, None)
    } else {
        let (input, _) = space1(input)?;
        let (input, version) = take_till1(|c: char| c.is_whitespace() || c == '=')(input)?;
        (input, Some(version))
    };

    // Consume "=>" with surrounding spaces.
    let (input, _) = tuple((space1, tag("=>"), space1))(input)?;

    // Parse the destination path and optional version.
    let (input, (dest_path, dest_version)) = tuple((
        take_till1(|c: char| c.is_whitespace()),
        opt(preceded(space1, take_till1(|c: char| c.is_whitespace()))),
    ))(input)?;

    let replacement = if let Some(version) = dest_version {
        Replacement::Module(Module {
            path: dest_path.to_string(),
            version: version.trim().to_string(),
            indirect: false,
        })
    } else {
        Replacement::FilePath(dest_path.to_string())
    };

    Ok((input, ModuleReplacement {
        path: src_path.to_string(),
        version: src_version.map(|s| s.trim().to_string()),
        replacement,
    }))
}

fn exclude_directive(input: &str) -> IResult<&str, Directive> {
    preceded(tuple((tag("exclude"), space1)), alt((module_block, map(require_spec, |r| vec![r]))))(
        input,
    )
    .map(|(next_input, deps)| (next_input, Directive::Exclude(deps)))
}

fn parse_block<T, F>(input: &str, line_parser: F) -> IResult<&str, Vec<T>>
where
    F: Fn(&str) -> IResult<&str, T>,
{
    delimited(
        char('('),
        many0(preceded(multispace0, line_parser)),
        preceded(multispace0, char(')')),
    )(input)
}

fn module_block(input: &str) -> IResult<&str, Vec<Module>> {
    parse_block(input, require_spec)
}

fn replace_block(input: &str) -> IResult<&str, Vec<ModuleReplacement>> {
    parse_block(input, replace_spec)
}
