pub mod extension;

use std::{convert::TryFrom, path::PathBuf};

use crate::commands::{CommandResult, CommandValue, ExitCode};
pub use extension::*;
use log::{error, info};

use anyhow::Result;
use clap::{arg, ArgMatches, Command, ValueHint};

pub fn command<'a>() -> Command<'a> {
    Command::new("extension")
        .about("Run extensions")
        .subcommand(
            Command::new("add")
                .about("Install extension")
                .arg(arg!([PATH]).required(true).value_hint(ValueHint::FilePath)),
        )
        .subcommand(
            Command::new("remove")
                .about("Uninstall extension")
                .arg(arg!([NAME]).required(true)),
        )
        .subcommand(Command::new("list").about("List installed extensions"))
}

/// Generate the subcommands for each extension.
pub fn extensions_subcommands<'a>(command: Command<'a>) -> Command<'a> {
    let extensions = match list_extensions() {
        Ok(extensions) => extensions,
        Err(e) => {
            error!("Couldn't list extensions: {}", e);
            return command;
        }
    };

    let extensions = extensions
        .into_iter()
        .filter(|ext| {
            command
                .get_subcommands()
                .all(|p| p.get_name() != ext.name())
        })
        .collect::<Vec<_>>();

    extensions.into_iter().fold(command, |command, ext| {
        command.subcommand(Command::new(ext.name()))
    })
}

/// Entry point for the `extensions` subcommand.
pub async fn handle_extensions(matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("add") {
        subcmd_add_extension(matches.value_of_t("PATH")?).await
    } else if let Some(matches) = matches.subcommand_matches("remove") {
        subcmd_remove_extension(matches.value_of("NAME").unwrap()).await
    } else {
        // also covers the `list` case
        subcmd_list_extensions().await
    }
}

/// Handle the `extension add` subcommand path.
/// Add the extension from the specified path.
pub async fn subcmd_add_extension(path: PathBuf) -> CommandResult {
    let extension = Extension::try_from(path)?;
    info!("Installing extension {}...", extension.name());

    extension.install()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension remove` subcommand path.
/// Remove the extension named as specified.
pub async fn subcmd_remove_extension(name: &str) -> CommandResult {
    // let extension = Extension::try_from(extensions_path()?.join(name))?;
    let extension = Extension::load(name)?;

    extension.uninstall()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension` / `extension list` subcommand path.
/// List installed extensions.
pub async fn subcmd_list_extensions() -> CommandResult {
    let extensions = list_extensions()?;

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
fn list_extensions() -> Result<Vec<Extension>> {
    let extension_path = extensions_path()?;

    if !extension_path.exists() {
        return Ok(Vec::new());
    }

    Ok(std::fs::read_dir(extension_path)?
        .filter_map(|d| Extension::try_from(d.map(|d| d.path()).ok()?).ok())
        .collect::<Vec<_>>())
}
