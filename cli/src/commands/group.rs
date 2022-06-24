//! Subcommand `phylum group`.

use clap::ArgMatches;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::{print, print_user_failure, print_user_success};

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &mut PhylumApi, mut matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        let group_name = matches.value_of("group_name").unwrap();
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

        let response = api.get_groups_list().await;

        let pretty_print = !matches.is_present("json");
        print::print_response(&response, pretty_print, None);

        Ok(ExitCode::Ok.into())
    }
}
