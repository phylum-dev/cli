//! Subcommand `phylum firewall`.

use std::str::FromStr;

use clap::ArgMatches;
use purl::{PackageType, Purl};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::config::Config;
use crate::format::Format;
use crate::print_user_failure;
use crate::types::{FirewallAction, FirewallLogFilter};

/// Handle `phylum firewall` subcommand.
pub async fn handle_firewall(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    match matches.subcommand() {
        Some(("log", matches)) => handle_log(api, matches, config).await,
        _ => unreachable!("invalid clap configuration"),
    }
}

/// Handle `phylum firewall log` subcommand.
pub async fn handle_log(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    let org = config.org();
    let group = matches.get_one::<String>("group").unwrap();

    // Get log filter args.
    let package_type = matches.get_one::<String>("package-type");
    let action = matches.get_one::<FirewallAction>("action");
    let before = matches.get_one::<String>("before");
    let after = matches.get_one::<String>("after");
    let purl = matches.get_one::<String>("purl");
    let limit = matches.get_one::<i64>("limit").unwrap();

    // Parse PURL filter.
    let parsed_purl = purl.map(|purl| Purl::from_str(purl));
    let (package_type, namespace, name, version) = match &parsed_purl {
        Some(Ok(purl)) => {
            (Some(*purl.package_type()), purl.namespace(), Some(purl.name()), purl.version())
        },
        Some(Err(err)) => {
            print_user_failure!("Could not parse purl {purl:?}: {err}");
            return Ok(ExitCode::Generic);
        },
        None => {
            let package_type = package_type.and_then(|pt| PackageType::from_str(pt).ok());
            (package_type, None, None, None)
        },
    };

    // Construct the filter.
    let filter = FirewallLogFilter {
        namespace,
        version,
        name,
        before: before.map(String::as_str),
        after: after.map(String::as_str),
        limit: Some(*limit as i32),
        ecosystem: package_type,
        action: action.copied(),
    };

    let response = api.firewall_log(org, group, filter).await?;

    let pretty = !matches.get_flag("json");
    response.data.write_stdout(pretty);

    Ok(ExitCode::Ok)
}
