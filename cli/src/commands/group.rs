//! Subcommand `phylum group`.

use std::cmp::Ordering;

use clap::ArgMatches;
use reqwest::StatusCode;
use serde::Serialize;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::config::Config;
use crate::format::Format;
use crate::{print_user_failure, print_user_success};

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    match matches.subcommand() {
        Some(("list", matches)) => handle_group_list(api, matches).await,
        Some(("create", matches)) => handle_group_create(api, matches, config).await,
        Some(("delete", matches)) => handle_group_delete(api, matches, config).await,
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
pub async fn handle_group_create(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    let group_name = matches.get_one::<String>("group_name").unwrap();

    let org = config.org();
    let response = if let Some(org) = org {
        api.org_create_group(org, group_name).await
    } else {
        api.create_group(group_name).await.map(|_| ())
    };

    match response {
        Ok(_) => {
            print_user_success!("Successfully created group {}", group_name);
            Ok(ExitCode::Ok)
        },
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            print_user_failure!("Group '{}' already exists", group_name);
            Ok(ExitCode::AlreadyExists)
        },
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::FORBIDDEN, .. }))
            if org.is_some() =>
        {
            print_user_failure!("Authorization failed, only organization admins can create groups");
            Ok(ExitCode::NotAuthenticated)
        },
        Err(err) => Err(err.into()),
    }
}

/// Handle `phylum group delete` subcommand.
pub async fn handle_group_delete(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    let group_name = matches.get_one::<String>("group_name").unwrap();

    if let Some(org) = config.org() {
        let response = api.org_delete_group(org, group_name).await;
        if let Err(PhylumApiError::Response(ResponseError {
            code: StatusCode::FORBIDDEN, ..
        })) = response
        {
            print_user_failure!("Authorization failed, only organization admins can delete groups");
            return Ok(ExitCode::NotAuthenticated);
        }
    } else {
        api.delete_group(group_name).await?;
    };

    print_user_success!("Successfully deleted group {}", group_name);

    Ok(ExitCode::Ok)
}

/// Handle `phylum group list` subcommand.
pub async fn handle_group_list(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    // Get org groups.
    let mut groups = Vec::new();
    match matches.get_one::<String>("org") {
        // If org is explicitly specified, only show its groups.
        Some(org_name) => {
            for group in api.org_groups(org_name).await?.groups {
                groups.push(ListGroupsEntry { org: Some(org_name.clone()), name: group.name });
            }
        },
        // If org is not specified as CLI arg, print all org and and legacy groups.
        None => {
            let legacy_groups = api.get_groups_list().await?.groups;
            groups = legacy_groups
                .into_iter()
                .map(|group| ListGroupsEntry { name: group.group_name, org: None })
                .collect();

            for org in api.orgs().await?.organizations {
                for group in api.org_groups(&org.name).await?.groups {
                    groups.push(ListGroupsEntry { org: Some(org.name.clone()), name: group.name });
                }
            }
        },
    }

    // Sort response for more consistent output.
    groups.sort_unstable_by(|a, b| match a.org.cmp(&b.org) {
        Ordering::Equal => a.name.cmp(&b.name),
        ordering => ordering,
    });

    let pretty = !matches.get_flag("json");
    groups.write_stdout(pretty);

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

/// Output entry in the `phylum group list` subcommand.
#[derive(Serialize)]
pub struct ListGroupsEntry {
    pub org: Option<String>,
    pub name: String,
}
