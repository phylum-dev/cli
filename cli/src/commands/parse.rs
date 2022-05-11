//! `phylum parse` command for lockfile parsing

use std::io;
use std::path::Path;

use anyhow::Result;
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::lock_files;
use super::{CommandResult, ExitCode};
use crate::lockfiles::*;

type ParserResult = Result<(Vec<PackageDescriptor>, PackageType)>;

const LOCKFILE_PARSERS: &[(&str, &dyn Fn(&Path) -> ParserResult)] = &[
    ("yarn", &lock_files::parse::<YarnLock>),
    ("npm", &lock_files::parse::<PackageLock>),
    ("gem", &lock_files::parse::<GemLock>),
    ("pip", &lock_files::parse::<PyRequirements>),
    ("pipenv", &lock_files::parse::<PipFile>),
    ("poetry", &lock_files::parse::<Poetry>),
    ("mvn", &lock_files::parse::<Pom>),
    ("gradle", &lock_files::parse::<GradleDeps>),
    ("nuget", &lock_files::parse::<CSProj>),
    ("auto", &lock_files::get_packages_from_lockfile),
];

pub fn lockfile_types() -> Vec<&'static str> {
    LOCKFILE_PARSERS.iter().map(|(name, _)| *name).collect()
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let lockfile_type = matches.value_of("lockfile-type").unwrap_or("auto");
    // LOCKFILE is a required parameter, so .unwrap() should be safe.
    let lockfile = matches.value_of("LOCKFILE").unwrap();

    let parser = LOCKFILE_PARSERS
        .iter()
        .filter_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
        .next()
        .unwrap();

    let (pkgs, _) = parser(Path::new(lockfile))?;

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok.into())
}
