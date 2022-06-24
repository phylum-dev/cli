use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{arg, ArgAction, ArgMatches, Command, ValueHint};
use deno_runtime::permissions::PermissionsOptions;
use dialoguer::console::Term;
use dialoguer::Confirm;
use extension::Extension;
use futures::future::BoxFuture;
use log::{error, warn};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, CommandValue, ExitCode};

pub mod api;
pub mod extension;
pub mod permissions;

pub fn command<'a>() -> Command<'a> {
    Command::new("extension")
        .about("Run extensions")
        .subcommand(
            Command::new("add")
                .about("Install extension")
                .arg(
                    arg!(-y --yes "Automatically accept requested permissions")
                        .action(ArgAction::SetTrue),
                )
                .arg(arg!([PATH]).required(true).value_hint(ValueHint::DirPath)),
        )
        .subcommand(
            Command::new("remove").about("Uninstall extension").arg(arg!([NAME]).required(true)),
        )
        .subcommand(Command::new("list").about("List installed extensions"))
}

/// Generate the subcommands for each extension.
/// TODO add tests.
pub fn add_extensions_subcommands(command: Command<'_>) -> Command<'_> {
    let extensions = match installed_extensions() {
        Ok(extensions) => extensions,
        Err(e) => {
            error!("Couldn't list extensions: {}", e);
            return command;
        },
    };

    let names = command.get_subcommands().map(|n| n.get_name().to_string()).collect::<HashSet<_>>();

    extensions
        .into_iter()
        .filter(|ext| {
            if names.contains(ext.name()) {
                warn!("{}: extension was filtered out due to name conflict", ext.name());
                false
            } else {
                true
            }
        })
        .fold(command, |command, ext| command.subcommand(Command::new(ext.name())))
}

/// Entry point for the `extensions` subcommand.
pub async fn handle_extensions(matches: &ArgMatches) -> CommandResult {
    match matches.subcommand() {
        Some(("add", matches)) => {
            handle_add_extension(
                matches.value_of("PATH").unwrap(),
                matches.get_one::<bool>("yes").copied().unwrap_or(false),
            )
            .await
        },
        Some(("remove", matches)) => {
            handle_remove_extension(matches.value_of("NAME").unwrap()).await
        },
        Some(("list", _)) | None => handle_list_extensions().await,
        _ => unreachable!(),
    }
}

/// Handle the `<extension>` command path.
///
/// Run the extension by name.
pub async fn handle_run_extension(
    name: &str,
    api: BoxFuture<'static, Result<PhylumApi>>,
) -> CommandResult {
    let extension = Extension::load(name)?;

    extension.run(api).await?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension add` subcommand path.
///
/// Add the extension from the specified path.
async fn handle_add_extension(path: &str, accept_permissions: bool) -> CommandResult {
    // NOTE: Extension installation without slashes is reserved for the marketplace.
    if !path.contains('/') && !path.contains('\\') {
        return Err(anyhow!("Ambiguous extension URI '{}', use './{0}' instead", path));
    }

    let extension_path = PathBuf::from(path);
    let extension = Extension::try_from(extension_path)?;

    let permissions = extension.permissions();

    // Attempt to construct a `PermissionsOptions` from the `Permissions`
    // object in order to validate the permissions.
    let _ = PermissionsOptions::try_from(permissions)?;

    if !accept_permissions && extension.requires_permissions() {
        println!("The `{}` extension requires the following permissions:", extension.name());

        if let Some(read_paths) = permissions.read() {
            println!("  Read from the following paths:");
            for path in read_paths {
                println!("    {}", path);
            }
        }

        if let Some(write_paths) = permissions.write() {
            println!("  Write to the following paths:");
            for path in write_paths {
                println!("    {}", path);
            }
        }

        if let Some(run_commands) = permissions.run() {
            println!("  Run the following commands:");
            for cmd in run_commands {
                println!("    {}", cmd);
            }
        }

        if let Some(access_urls) = permissions.net() {
            println!("  Access the following URLs:");
            for url in access_urls {
                println!("    {}", url);
            }
        }

        if !Term::stdout().is_term() {
            return Err(anyhow!("can't ask for permissions: not a terminal"));
        }

        if !Confirm::new().with_prompt("Do you accept?").interact()? {
            return Err(anyhow!("permissions not granted, aborting"));
        }
    }

    extension.install()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension remove` subcommand path.
///
/// Remove the extension named as specified.
async fn handle_remove_extension(name: &str) -> CommandResult {
    let extension = Extension::load(name)?;

    extension.uninstall()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension` / `extension list` subcommand path.
///
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

/// Return a list of installed extensions. Filter out invalid extensions instead
/// of exiting early.
pub fn installed_extensions() -> Result<Vec<Extension>> {
    let extensions_path = extension::extensions_path()?;

    let dir_entry = match fs::read_dir(extensions_path) {
        Ok(d) => d,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                return Ok(Vec::new());
            } else {
                return Err(e.into());
            }
        },
    };

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
                },
            }
        })
        .collect::<Vec<_>>())
}
