//! Subcommand `phylum group`.

use anyhow::anyhow;
use clap::ArgMatches;
use dialoguer::Confirm;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::format::Format;
use crate::{print_user_failure, print_user_success, print_user_warning};

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        handle_group_create(api, matches).await
    } else if let Some(matches) = matches.subcommand_matches("transfer") {
        handle_group_transfer(api, matches).await
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

/// Handle `phylum group transfer` subcommand.
pub async fn handle_group_transfer(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    let group = matches.get_one::<String>("group").unwrap();
    let user = matches.get_one::<String>("user").unwrap();

    if !matches.get_flag("force") {
        // Prompt user to avoid accidental transfer.
        let should_continue = Confirm::new()
            .with_prompt(format!(
                "This will transfer ownership of `{group}` to `{user}`. You will no longer own \
                 the group. Are you sure?"
            ))
            .default(false)
            .interact()?;

        // Abort if user did not confirm our prompt.
        if !should_continue {
            print_user_warning!("Aborting group transfer");
            return Ok(ExitCode::ConfirmationFailed.into());
        }
    }

    // Transfer ownership.
    match api.group_set_owner(group, user).await {
        // Improve error message for invalid groups.
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::NOT_FOUND, .. })) => {
            return Err(anyhow!(format!("Group `{group}` does not exist")));
        },
        result => result,
    }?;

    print_user_success!("Successfully transferred group ownership!");

    Ok(ExitCode::Ok.into())
}
