//! Subcommand `phylum firewall`.

use std::borrow::Cow;
use std::str::FromStr;

use clap::ArgMatches;
use purl::Purl;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::config::Config;
use crate::format::Format;
use crate::print_user_failure;
use crate::types::FirewallLogFilter;

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
    let ecosystem = matches.get_one::<String>("ecosystem");
    let purl = matches.get_one::<String>("package");
    let action = matches.get_one::<String>("action");
    let before = matches.get_one::<String>("before");
    let after = matches.get_one::<String>("after");
    let limit = matches.get_one::<i64>("limit").unwrap();

    // Parse PURL filter.
    let parsed_purl = purl.map(|purl| Purl::from_str(purl));
    let (ecosystem, namespace, name, version) = match &parsed_purl {
        Some(Ok(purl)) => {
            let ecosystem = Cow::Owned(purl.package_type().to_string());
            (Some(ecosystem), purl.namespace(), Some(purl.name()), purl.version())
        },
        Some(Err(err)) => {
            print_user_failure!("Could not parse purl {purl:?}: {err}");
            return Ok(ExitCode::Generic);
        },
        None => (ecosystem.map(Cow::Borrowed), None, None, None),
    };

    // Construct the filter.
    let filter = FirewallLogFilter {
        ecosystem: ecosystem.as_ref().map(|e| e.as_str()),
        namespace,
        name,
        version,
        action: action.map(String::as_str),
        before: before.map(String::as_str),
        after: after.map(String::as_str),
        limit: Some(*limit as i32),
    };

    let response = api.firewall_log(org, group, filter).await?;

    let pretty = !matches.get_flag("json");
    response.data.write_stdout(pretty);

    Ok(ExitCode::Ok)
}
