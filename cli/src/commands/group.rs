//! Subcommand `phylum group`.

use clap::ArgMatches;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::format::Format;
use crate::{print_user_failure, print_user_success};

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        handle_group_create(api, matches).await
    } else if let Some(matches) = matches.subcommand_matches("member") {
        let group = matches.get_one::<String>("group").unwrap();

        if let Some(matches) = matches.subcommand_matches("add") {
            handle_member_add(api, matches, group).await
        } else if let Some(matches) = matches.subcommand_matches("remove") {
            handle_member_remove(api, matches, group).await
        } else {
            handle_member_list(api, matches, group).await
        }
    } else {
        handle_group_list(api, matches).await
    }
}

/// Handle `phylum group create` subcommand.
pub async fn handle_group_create(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    let group_name = matches.get_one::<String>("group_name").unwrap();
    match api.create_group(group_name).await {
        Ok(response) => {
            print_user_success!("Successfully created group {}", response.group_name);
            Ok(ExitCode::Ok.into())
        },
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            print_user_failure!("Group '{}' already exists", group_name);
            Ok(ExitCode::AlreadyExists.into())
        },
        Err(err) => Err(err.into()),
    }
}

/// Handle `phylum group list` subcommand.
pub async fn handle_group_list(api: &mut PhylumApi, mut matches: &ArgMatches) -> CommandResult {
    matches = matches.subcommand_matches("list").unwrap_or(matches);
    let pretty = !matches.get_flag("json");

    let response = api.get_groups_list().await?;

    response.write_stdout(pretty);

    Ok(ExitCode::Ok.into())
}

/// Handle `phylum group member add` subcommand.
pub async fn handle_member_add(
    api: &mut PhylumApi,
    matches: &ArgMatches,
    group: &str,
) -> CommandResult {
    let users = matches.get_many::<String>("user").unwrap();

    for user in users {
        api.group_add(group, user).await?;
        print_user_success!("Successfully added {user:?} to group {group:?}");
    }

    Ok(ExitCode::Ok.into())
}

/// Handle `phylum group member remove` subcommand.
pub async fn handle_member_remove(
    api: &mut PhylumApi,
    matches: &ArgMatches,
    group: &str,
) -> CommandResult {
    let users = matches.get_many::<String>("user").unwrap();

    for user in users {
        api.group_remove(group, user).await?;
        print_user_success!("Successfully removed {user:?} from group {group:?}");
    }

    Ok(ExitCode::Ok.into())
}

/// Handle `phylum group member` subcommand.
pub async fn handle_member_list(
    api: &mut PhylumApi,
    mut matches: &ArgMatches,
    group: &str,
) -> CommandResult {
    matches = matches.subcommand_matches("list").unwrap_or(matches);
    let pretty = !matches.get_flag("json");

    let response = api.group_members(group).await?;

    response.write_stdout(pretty);

    Ok(ExitCode::Ok.into())
}
