use std::str::FromStr;

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use console::style;
use phylum_types::types::package::{PackageDescriptor, PackageType};
use reqwest::StatusCode;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::filter::{Filter, FilterIssues};
use crate::format::Format;
use crate::print_user_warning;

fn parse_package(matches: &ArgMatches, request_type: PackageType) -> Result<PackageDescriptor> {
    // Read required options.
    let name = matches.get_one::<String>("name").unwrap().to_string();
    let version = matches.get_one::<String>("version").unwrap().to_string();

    // If a package type was provided on the command line, prefer that
    // to the global setting
    let package_type = match matches.get_one::<String>("package-type") {
        Some(pt) => {
            PackageType::from_str(pt).map_err(|_| anyhow!("invalid package type: {}", pt))?
        },
        None => request_type,
    };

    Ok(PackageDescriptor { name, version, package_type })
}

/// Handle the subcommands for the `package` subcommand.
pub async fn handle_get_package(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let pretty_print = !matches.get_flag("json");

    let pkg = parse_package(matches, api.config().request_type)?;
    let mut resp = match api.get_package_details(&pkg).await {
        Ok(resp) => resp,
        Err(err) if err.status() == Some(StatusCode::NOT_FOUND) => {
            print_user_warning!(
                "No matching packages found. Submit a lockfile for processing:\n\n\t{}\n",
                style("phylum analyze <lock_file>").blue()
            );
            return Ok(ExitCode::PackageNotFound.into());
        },
        Err(err) => return Err(err.into()),
    };

    let filter = matches.get_one::<String>("filter").and_then(|v| Filter::from_str(v).ok());
    if let Some(filter) = filter {
        resp.filter(&filter);
    }

    resp.write_stdout(pretty_print);

    Ok(ExitCode::Ok.into())
}
