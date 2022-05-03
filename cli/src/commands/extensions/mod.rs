use std::{convert::TryFrom, fs::File, io::Read, path::PathBuf};

use crate::commands::{CommandResult, CommandValue, ExitCode};

use anyhow::{anyhow, Result};
use clap::{arg, ArgMatches, Command, ValueHint};
use serde::Deserialize;

const MANIFEST_NAME: &str = "PhylumExt.toml";

#[derive(Debug)]
pub struct Extension {
    path: PathBuf,
    manifest: ExtensionManifest,
}

#[derive(Deserialize, Debug)]
pub struct ExtensionManifest {
    name: String,
    description: Option<String>,
    entry_point: String,
}

// Load the extension from the specified path.
impl TryFrom<PathBuf> for Extension {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.is_dir() {
            return Err(anyhow!("{}: not a directory", path.to_string_lossy()));
        }

        let manifest_path = path.join(MANIFEST_NAME);
        if !manifest_path.exists() {
            return Err(anyhow!(
                "{}: missing {}",
                path.to_string_lossy(),
                MANIFEST_NAME
            ));
        }

        let mut buf = Vec::new();
        File::open(manifest_path)?.read_to_end(&mut buf)?;

        let manifest: ExtensionManifest = toml::from_slice(&buf)?;
        let entry_point_path = path.join(&manifest.entry_point);

        if !entry_point_path.exists() {
            return Err(anyhow!(
                "{}: entry point does not exist",
                entry_point_path.to_string_lossy()
            ));
        }

        if !entry_point_path.is_file() {
            return Err(anyhow!(
                "{}: entry point is not a file",
                entry_point_path.to_string_lossy()
            ));
        }

        // TODO add further validation if necessary:
        // - Check that the name matches /^[a-z0-9-_]+$/
        // - Check that the entry point is a supported format (.wasm?)
        // - Check that the entry point is appropriately signed
        Ok(Extension { path, manifest })
    }
}

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
pub fn extensions_subcommands<'a>(command: Command<'a>) -> Result<Command<'a>> {
    let extensions = list_extensions()?
        .into_iter()
        .filter(|ext| {
            command
                .get_subcommands()
                .all(|p| p.get_name() != ext.manifest.name)
        })
        .collect::<Vec<_>>();

    Ok(extensions.into_iter().fold(command, |command, ext| {
        command.subcommand(Command::new(&ext.manifest.name))
    }))
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

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension remove` subcommand path.
/// Remove the extension named as specified.
pub async fn subcmd_remove_extension(name: &str) -> CommandResult {
    let extension = Extension::try_from(extensions_path()?.join(name))?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension` / `extension list` subcommand path.
/// List installed extensions.
pub async fn subcmd_list_extensions() -> CommandResult {
    let extensions = list_extensions()?;

    extensions.into_iter().for_each(|ext| {
        println!(
            "{:20}   {}",
            ext.manifest.name,
            ext.manifest
                .description
                .as_ref()
                .map(String::as_str)
                .unwrap_or("")
        );
    });

    Ok(CommandValue::Code(ExitCode::Ok))
}

// Return a list of installed extensions. Filter out invalid extensions instead of exiting early.
fn list_extensions() -> Result<Vec<Extension>> {
    Ok(std::fs::read_dir(extensions_path()?)?
        .filter_map(|d| Extension::try_from(d.map(|d| d.path()).ok()?).ok())
        .collect::<Vec<_>>())
}

// Construct and return the extension path: $XDG_DATA_HOME/phylum/extensions
fn extensions_path() -> Result<PathBuf, anyhow::Error> {
    Ok(crate::config::data_dir()?.join("phylum").join("extensions"))
}
