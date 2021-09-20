use std::str::FromStr;

use ansi_term::Color::Blue;

use clap::ArgMatches;
use phylum_cli::api::PhylumApi;
use phylum_cli::types::{PackageDescriptor, PackageType};

use crate::print::print_response;
use crate::print_user_warning;

fn parse_package(options: &ArgMatches, request_type: &PackageType) -> Option<PackageDescriptor> {
    if !(options.is_present("name") && options.is_present("version")) {
        return None;
    }

    let name = options.value_of("name").unwrap().to_string(); // required option
    let version = options.value_of("version").unwrap().to_string();
    let mut r#type = request_type.to_owned();

    // If a package type was provided on the command line, prefer that
    //  to the global setting
    if options.is_present("type") {
        r#type = PackageType::from_str(options.value_of("type").unwrap()).unwrap_or(r#type);
    }

    Some(PackageDescriptor {
        name,
        version,
        r#type,
    })
}

/// Handle the subcommands for the `package` subcommand.
pub fn handle_get_package(
    api: &mut PhylumApi,
    req_type: &PackageType,
    matches: &clap::ArgMatches,
) -> u8 {
    let pretty_print = !matches.is_present("json");
    let pkg = parse_package(matches, req_type);
    if pkg.is_none() {
        return 1;
    }
    let resp = api.get_package_details(&pkg.unwrap());
    log::debug!("==> {:?}", resp);

    if let Err(phylum_cli::Error::HttpError(404, _)) = resp {
        print_user_warning!(
            "No matching packages found. Submit a lockfile for processing:\n\n\t{}\n",
            Blue.paint("phylum analyze <lock_file>")
        );
    } else {
        print_response(&resp, pretty_print);
    }

    0
}
