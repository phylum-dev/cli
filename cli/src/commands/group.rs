//! Subcommand `phylum group`.

use clap::ArgMatches;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::format::Format;
use crate::{print_user_failure, print_user_success};

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &mut PhylumApi, mut matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
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
    } else {
        matches = matches.subcommand_matches("list").unwrap_or(matches);

        let response = api.get_groups_list().await?;

        let pretty = !matches.contains_id("json");
        response.write_stdout(pretty);

        Ok(ExitCode::Ok.into())
    }
}
