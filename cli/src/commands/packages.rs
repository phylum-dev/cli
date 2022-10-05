use std::str::FromStr;

use clap::ArgMatches;
use console::style;
use phylum_types::types::package::{PackageDescriptor, PackageType};
use reqwest::StatusCode;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::filter::{Filter, FilterIssues};
use crate::format::Format;
use crate::print_user_warning;

fn parse_package(options: &ArgMatches, request_type: &PackageType) -> PackageDescriptor {
    // Read required options.
    let name = options.value_of("name").unwrap().to_string();
    let version = options.value_of("version").unwrap().to_string();

    let mut package_type = request_type.to_owned();

    // If a package type was provided on the command line, prefer that
    //  to the global setting
    if options.is_present("package-type") {
        package_type = PackageType::from_str(options.value_of("package-type").unwrap())
            .unwrap_or(package_type);
    }

    PackageDescriptor { name, version, package_type }
}

/// Handle the subcommands for the `package` subcommand.
pub async fn handle_get_package(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let pretty_print = !matches.is_present("json");

    let pkg = parse_package(matches, &api.config().request_type);
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

    let filter = matches.value_of("filter").and_then(|v| Filter::from_str(v).ok());
    if let Some(filter) = filter {
        resp.filter(&filter);
    }

    resp.write_stdout(pretty_print);

    Ok(ExitCode::Ok.into())
}
