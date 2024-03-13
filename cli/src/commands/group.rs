//! Subcommand `phylum group`.

use clap::ArgMatches;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::format::Format;
use crate::{print_user_failure, print_user_success};

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    match matches.subcommand() {
        Some(("list", matches)) => handle_group_list(api, matches).await,
        Some(("create", matches)) => handle_group_create(api, matches).await,
        Some(("delete", matches)) => handle_group_delete(api, matches).await,
        Some(("member", matches)) => {
            let group = matches.get_one::<String>("group").unwrap();

            match matches.subcommand() {
                Some(("list", matches)) => handle_member_list(api, matches, group).await,
                Some(("add", matches)) => handle_member_add(api, matches, group).await,
                Some(("remove", matches)) => handle_member_remove(api, matches, group).await,
                _ => unreachable!("invalid clap configuration"),
            }
        },
        _ => unreachable!("invalid clap configuration"),
    }
}

/// Handle `phylum group create` subcommand.
pub async fn handle_group_create(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    let group_name = matches.get_one::<String>("group_name").unwrap();
    match api.create_group(group_name).await {
        Ok(response) => {
            print_user_success!("Successfully created group {}", response.group_name);
            Ok(ExitCode::Ok)
        },
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            print_user_failure!("Group '{}' already exists", group_name);
            Ok(ExitCode::AlreadyExists)
        },
        Err(err) => Err(err.into()),
    }
}

/// Handle `phylum group delete` subcommand.
pub async fn handle_group_delete(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    let group_name = matches.get_one::<String>("group_name").unwrap();
    api.delete_group(group_name).await?;

    print_user_success!("Successfully deleted group {}", group_name);

    Ok(ExitCode::Ok)
}

/// Handle `phylum group list` subcommand.
pub async fn handle_group_list(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    let response = api.get_groups_list().await?;

    let pretty = !matches.get_flag("json");
    response.write_stdout(pretty);

    Ok(ExitCode::Ok)
}

/// Handle `phylum group member add` subcommand.
pub async fn handle_member_add(
    api: &PhylumApi,
    matches: &ArgMatches,
    group: &str,
) -> CommandResult {
    let users = matches.get_many::<String>("user").unwrap();

    for user in users {
        api.group_add(group, user).await?;
        print_user_success!("Successfully added {user:?} to group {group:?}");
    }

    Ok(ExitCode::Ok)
}

/// Handle `phylum group member remove` subcommand.
pub async fn handle_member_remove(
    api: &PhylumApi,
    matches: &ArgMatches,
    group: &str,
) -> CommandResult {
    let users = matches.get_many::<String>("user").unwrap();

    for user in users {
        api.group_remove(group, user).await?;
        print_user_success!("Successfully removed {user:?} from group {group:?}");
    }

    Ok(ExitCode::Ok)
}

/// Handle `phylum group member` subcommand.
pub async fn handle_member_list(
    api: &PhylumApi,
    matches: &ArgMatches,
    group: &str,
) -> CommandResult {
    let response = api.group_members(group).await?;

    let pretty = !matches.get_flag("json");
    response.write_stdout(pretty);

    Ok(ExitCode::Ok)
}
