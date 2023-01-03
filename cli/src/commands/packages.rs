use std::str::FromStr;

use anyhow::Result;
use clap::ArgMatches;
use phylum_types::types::package::{PackageSpecifier, PackageSubmitResponse, PackageType};
use reqwest::StatusCode;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::filter::{Filter, FilterIssues};
use crate::format::Format;
use crate::print_user_warning;

fn parse_package(matches: &ArgMatches, request_type: PackageType) -> Result<PackageSpecifier> {
    // Read required options.
    let name = matches.get_one::<String>("name").unwrap().to_string();
    let version = matches.get_one::<String>("version").unwrap().to_string();

    // If a package type was provided on the command line, prefer that
    // to the global setting
    let registry = match matches.get_one::<String>("package-type") {
        Some(package_type) => package_type.clone(),
        None => request_type.to_string(),
    };

    Ok(PackageSpecifier { name, version, registry })
}

/// Handle the subcommands for the `package` subcommand.
pub async fn handle_get_package(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let pretty_print = !matches.get_flag("json");

    let pkg = parse_package(matches, api.config().request_type)?;
    let resp = match api.submit_package(&pkg).await {
        Ok(resp) => resp,
        Err(err) if err.status() == Some(StatusCode::NOT_FOUND) => {
            print_user_warning!("No matching package found.");
            return Ok(ExitCode::PackageNotFound.into());
        },
        Err(err) => return Err(err.into()),
    };

    match resp {
        PackageSubmitResponse::AlreadyProcessed(mut resp) => {
            let filter = matches.get_one::<String>("filter").and_then(|v| Filter::from_str(v).ok());
            if let Some(filter) = filter {
                resp.filter(&filter);
            }

            resp.write_stdout(pretty_print);
        },
        PackageSubmitResponse::AlreadySubmitted => {
            print_user_warning!(
                "Package is still processing. Please check back later for results."
            );
        },
        PackageSubmitResponse::New => {
            print_user_warning!(
                "Thank you for submitting this package. Please check back later for results."
            );
        },
    }

    Ok(ExitCode::Ok.into())
}
