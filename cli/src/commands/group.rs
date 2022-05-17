//! Subcommand `phylum group`.

use clap::ArgMatches;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::print;
use crate::print_user_success;

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        let group_name = matches.value_of("group_name").unwrap();
        let response = api.create_group(group_name).await?;
        print_user_success!("Successfully created group {}", response.group_name);
        Ok(ExitCode::Ok.into())
    } else {
        let response = api.get_groups_list().await;

        let pretty_print = !matches.is_present("json");
        print::print_response(&response, pretty_print, None);

        Ok(ExitCode::Ok.into())
    }
}
