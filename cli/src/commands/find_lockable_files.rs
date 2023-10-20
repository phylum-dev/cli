//! `phylum find-lockable-files` subcommand.

use crate::commands::{CommandResult, ExitCode};

/// Handle `phylum find-lockable-files` subcommand.
pub fn handle_command() -> CommandResult {
    let lockables = phylum_lockfile::LockableFiles::find_at(".");
    let json = serde_json::to_string(&lockables)?;
    println!("{}", json);
    Ok(ExitCode::Ok)
}
