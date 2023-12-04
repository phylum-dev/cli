//! `phylum find-lockable-files` subcommand.

use crate::commands::{CommandResult, ExitCode};

/// Handle `phylum find-lockable-files` subcommand.
pub fn handle_command() -> CommandResult {
    let depfiles = phylum_lockfile::DepFiles::find_at(".");
    let json = serde_json::to_string(&depfiles)?;
    println!("{}", json);
    Ok(ExitCode::Ok)
}
