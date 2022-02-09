use std::str::FromStr;

use ansi_term::Color::Blue;
use anyhow::anyhow;
use reqwest::StatusCode;
use phylum_types::types::package::*;

use clap::ArgMatches;
use phylum_cli::api::PhylumApi;

use crate::commands::{CommandResult, CommandValue};
use crate::print::print_response;
use crate::print_user_warning;

fn parse_package(options: &ArgMatches, request_type: &PackageType) -> Option<PackageDescriptor> {
    if !(options.is_present("name") && options.is_present("version")) {
        return None;
    }

    let name = options.value_of("name").unwrap().to_string(); // required option
    let version = options.value_of("version").unwrap().to_string();
    let mut package_type = request_type.to_owned();

    // If a package type was provided on the command line, prefer that
    //  to the global setting
    if options.is_present("package-type") {
        package_type =
            PackageType::from_str(options.value_of("package-type").unwrap()).unwrap_or(package_type);
    }

    Some(PackageDescriptor {
        name,
        version,
        package_type,
    })
}

/// Handle the subcommands for the `package` subcommand.
pub async fn handle_get_package(
    api: &mut PhylumApi,
    req_type: &PackageType,
    matches: &clap::ArgMatches,
) -> CommandResult {
    let pretty_print = !matches.is_present("json");
    let pkg = parse_package(matches, req_type);
    if pkg.is_none() {
        return Err(anyhow!("Could not find or parse package information"));
    }
    let resp = api.get_package_details(&pkg.unwrap()).await;
    log::debug!("==> {:?}", resp);

    if let Err(Some(StatusCode::NOT_FOUND)) = resp.as_ref().map_err(|e| e.status()) {
        print_user_warning!(
            "No matching packages found. Submit a lockfile for processing:\n\n\t{}\n",
            Blue.paint("phylum analyze <lock_file>")
        );
    } else {
        print_response(&resp, pretty_print, None);
    }
    CommandValue::Void.into()
}
