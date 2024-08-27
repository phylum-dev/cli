//! Subcommand `phylum org`.

use clap::ArgMatches;
use reqwest::StatusCode;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::config::Config;
use crate::format::Format;
use crate::{print_user_failure, print_user_success, print_user_warning};

/// Handle `phylum org` subcommand.
pub async fn handle_org(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    match matches.subcommand() {
        Some(("list", matches)) => handle_org_list(api, matches).await,
        Some(("link", matches)) => handle_org_link(api, matches, config).await,
        Some(("unlink", _)) => handle_org_unlink(config).await,
        Some(("member", sub_matches)) => {
            let org = match config.org() {
                Some(org) => org,
                None => {
                    print_user_failure!("Arguments missing target organization");
                    print_user_failure!("Either add `--org` flag, or update your configuration:");
                    print_user_failure!("    phylum org link <ORG>");
                    return Ok(ExitCode::MissingOrg);
                },
            };

            match sub_matches.subcommand() {
                Some(("list", sub_matches)) => handle_member_list(api, sub_matches, org).await,
                Some(("add", sub_matches)) => handle_member_add(api, sub_matches, org).await,
                Some(("remove", sub_matches)) => handle_member_remove(api, sub_matches, org).await,
                _ => unreachable!("invalid clap configuration"),
            }
        },
        _ => unreachable!("invalid clap configuration"),
    }
}

/// Handle `phylum org list` subcommand.
pub async fn handle_org_list(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    let response = api.orgs().await?;

    let pretty = !matches.get_flag("json");
    response.write_stdout(pretty);

    Ok(ExitCode::Ok)
}

/// Handle `phylum org link` subcommand.
pub async fn handle_org_link(
    api: &PhylumApi,
    matches: &ArgMatches,
    mut config: Config,
) -> CommandResult {
    let org = matches.get_one::<String>("org").unwrap();

    // Attempt org access, to simplify troubleshooting.
    if api.org_members(org).await.is_err() {
        print_user_warning!(
            "Could not access organization {org:?}, future Phylum commands may fail unexpectedly"
        );
    }

    config.set_org(Some(org.into()));
    config.save()?;

    print_user_success!("Successfully set default organization to {org:?}");

    Ok(ExitCode::Ok)
}

/// Handle `phylum org unlink` subcommand.
pub async fn handle_org_unlink(mut config: Config) -> CommandResult {
    config.set_org(None);
    config.save()?;

    print_user_success!("Successfully cleared default organization");

    Ok(ExitCode::Ok)
}

/// Handle `phylum org member list` subcommand.
pub async fn handle_member_list(api: &PhylumApi, matches: &ArgMatches, org: &str) -> CommandResult {
    let response = api.org_members(org).await?;

    let pretty = !matches.get_flag("json");
    response.write_stdout(pretty);

    Ok(ExitCode::Ok)
}

/// Handle `phylum org member add` subcommand.
pub async fn handle_member_add(api: &PhylumApi, matches: &ArgMatches, org: &str) -> CommandResult {
    let users = matches.get_many::<String>("user").unwrap();

    for user in users {
        match api.org_member_add(org, user).await {
            Ok(()) => print_user_success!("Successfully added {user:?} to organization {org:?}"),
            Err(err) if err.status() == Some(StatusCode::CONFLICT) => {
                print_user_warning!("User {user:?} is already a member of organization {org:?}");
                return Ok(ExitCode::AlreadyExists);
            },
            Err(err) => return Err(err.into()),
        }
    }

    Ok(ExitCode::Ok)
}

/// Handle `phylum org member remove` subcommand.
pub async fn handle_member_remove(
    api: &PhylumApi,
    matches: &ArgMatches,
    org: &str,
) -> CommandResult {
    let users = matches.get_many::<String>("user").unwrap();

    for user in users {
        match api.org_member_remove(org, user).await {
            Ok(()) => {
                print_user_success!("Successfully removed {user:?} from organization {org:?}")
            },
            Err(err) if err.status() == Some(StatusCode::NOT_FOUND) => {
                print_user_warning!("User {user:?} is not a member of organization {org:?}");
                return Ok(ExitCode::NotFound);
            },
            Err(err) => return Err(err.into()),
        }
    }

    Ok(ExitCode::Ok)
}
