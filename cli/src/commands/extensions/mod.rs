pub mod extension;

use std::{collections::HashSet, convert::TryFrom, path::PathBuf};

use crate::commands::{CommandResult, CommandValue, ExitCode};
pub use extension::*;

use anyhow::Result;
use clap::{arg, ArgMatches, Command, ValueHint};
use log::{error, warn};

pub fn command<'a>() -> Command<'a> {
    Command::new("extension")
        .about("Run extensions")
        .subcommand(
            Command::new("add")
                .about("Install extension")
                .arg(arg!([PATH]).required(true).value_hint(ValueHint::DirPath)),
        )
        .subcommand(
            Command::new("remove")
                .about("Uninstall extension")
                .arg(arg!([NAME]).required(true)),
        )
        .subcommand(Command::new("list").about("List installed extensions"))
}

/// Generate the subcommands for each extension.
/// TODO add tests.
pub fn extensions_subcommands(command: Command<'_>) -> Command<'_> {
    let extensions = match installed_extensions() {
        Ok(extensions) => extensions,
        Err(e) => {
            error!("Couldn't list extensions: {}", e);
            return command;
        }
    };

    let names = command
        .get_subcommands()
        .map(|n| n.get_name().to_string())
        .collect::<HashSet<_>>();

    extensions
        .into_iter()
        .filter(|ext| {
            if names.contains(ext.name()) {
                warn!(
                    "{}: extension was filtered out due to name conflict",
                    ext.name()
                );
                false
            } else {
                true
            }
        })
        .fold(command, |command, ext| {
            command.subcommand(Command::new(ext.name()))
        })
}

/// Entry point for the `extensions` subcommand.
pub async fn handle_extensions(matches: &ArgMatches) -> CommandResult {
    match matches.subcommand() {
        Some(("add", matches)) => handle_add_extension(matches.value_of_t("PATH")?).await,
        Some(("remove", matches)) => handle_remove_extension(matches.value_of("NAME").unwrap()).await,
        Some(("list", _)) | None => handle_list_extensions().await,
        _ => unreachable!(),
    }
}

/// Handle the `extension add` subcommand path.
/// Add the extension from the specified path.
async fn handle_add_extension(path: PathBuf) -> CommandResult {
    let extension = Extension::try_from(path)?;

    extension.install()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension remove` subcommand path.
/// Remove the extension named as specified.
async fn handle_remove_extension(name: &str) -> CommandResult {
    let extension = Extension::load(name)?;

    extension.uninstall()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension` / `extension list` subcommand path.
/// List installed extensions.
async fn handle_list_extensions() -> CommandResult {
    let extensions = installed_extensions()?;

    if extensions.is_empty() {
        println!("No extensions are currently installed.");
    } else {
        extensions.into_iter().for_each(|ext| {
            println!("{:20}   {}", ext.name(), ext.description().unwrap_or(""));
        });
    }

    Ok(CommandValue::Code(ExitCode::Ok))
}

// Return a list of installed extensions. Filter out invalid extensions instead of exiting early.
fn installed_extensions() -> Result<Vec<Extension>> {
    let extensions_path = extensions_path()?;

    let dir_entry = match fs::read_dir(extensions_path) {
        Ok(d) => d,
        Err(e) => if e.kind() == ErrorKind::NotFound {
            return Ok(Vec::new());
        } else {
            return Err(e.into());
        }
    }

    Ok(dir_entry
        .filter_map(|dir_entry| {
            match dir_entry
                .map_err(|e| e.into())
                .and_then(|dir_entry| Extension::try_from(dir_entry.path()))
            {
                Ok(ext) => Some(ext),
                Err(e) => {
                    error!("{e}");
                    None
                }
            }
        })
        .collect::<Vec<_>>())
}
