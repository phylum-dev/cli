use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use ansi_term::Color;
use anyhow::{anyhow, Context, Result};
use clap::{arg, ArgMatches, Command, ValueHint};
use futures::future::BoxFuture;
use log::{error, warn};

use crate::api::PhylumApi;
use crate::commands::extensions::extension::{Extension, ExtensionManifest};
use crate::commands::{CommandResult, CommandValue, ExitCode};
use crate::print_user_success;

pub mod api;
pub mod extension;

const EXTENSION_SKELETON: &[u8] = b"\
import { PhylumApi } from 'phylum';

console.log('Hello, World!');
";

pub fn command<'a>() -> Command<'a> {
    Command::new("extension")
        .about("Run extensions")
        .subcommand(
            Command::new("install")
                .about("Install extension")
                .arg(arg!([PATH]).required(true).value_hint(ValueHint::DirPath)),
        )
        .subcommand(
            Command::new("uninstall").about("Uninstall extension").arg(arg!([NAME]).required(true)),
        )
        .subcommand(
            Command::new("new").about("Create a new extension").arg(arg!([PATH]).required(true)),
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
        Some(("install", matches)) => {
            handle_install_extension(matches.value_of("PATH").unwrap()).await
        },
        Some(("uninstall", matches)) => {
            handle_uninstall_extension(matches.value_of("NAME").unwrap()).await
        },
        Some(("new", matches)) => handle_create_extension(matches.value_of("PATH").unwrap()).await,
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

/// Handle the `extension install` subcommand path.
///
/// Install the extension from the specified path.
async fn handle_install_extension(path: &str) -> CommandResult {
    // NOTE: Extension installation without slashes is reserved for the marketplace.
    if !path.contains('/') && !path.contains('\\') {
        return Err(anyhow!("Ambiguous extension URI '{}', use './{0}' instead", path));
    }

    let extension_path = PathBuf::from(path);
    let extension = Extension::try_from(extension_path)?;

    extension.install()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension uninstall` subcommand path.
///
/// Uninstall the extension named as specified.
async fn handle_uninstall_extension(name: &str) -> CommandResult {
    let extension = Extension::load(name)?;

    extension.uninstall()?;

    Ok(CommandValue::Code(ExitCode::Ok))
}

/// Handle the `extension new` command path.
///
/// Create a new extension in the current directory.
pub async fn handle_create_extension(path: &str) -> CommandResult {
    // Error out when target is already occupied.
    //
    // This allows use to use [`fs::create_dir_all`] without having to worry about
    // reusing an existing directory.
    let extension_path = PathBuf::from(path);
    if extension_path.exists() {
        return Err(anyhow!("Destination {path:?} already exists"));
    }

    // Extract extension name.
    let name = extension_path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("Last segment in {path:?} is not a valid extension name"))?;

    // Create all missing directories.
    fs::create_dir_all(&extension_path)
        .with_context(|| format!("Unable to create all directories in {path:?}"))?;

    // Write manifest file.
    let manifest = ExtensionManifest::new(name.into(), "main.ts".into(), None);
    let manifest_path = extension_path.join("PhylumExt.toml");
    fs::write(manifest_path, toml::to_string(&manifest)?.as_bytes())?;

    // Create "Hello, World!" example.
    let entry_path = extension_path.join("main.ts");
    fs::write(entry_path, EXTENSION_SKELETON)?;

    print_user_success!(
        "\
        Extension created successfully
        \nRun `phylum extension add {path}` to install it."
    );

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
        let heading = Color::Blue.paint("Extension Name         Description");
        println!("{heading}");

        for extension in extensions {
            println!("{:20}   {}", extension.name(), extension.description().unwrap_or(""));
        }
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
