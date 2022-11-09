//! Subcommand `phylum init`.

use std::{env, io};

use clap::ArgMatches;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};

use crate::api::PhylumApi;
use crate::commands::{project, CommandResult, CommandValue, ExitCode};
use crate::{config, print_user_warning};

/// Handle `phylum init` subcommand.
pub async fn handle_init(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    // Prompt for confirmation if there already is a linked project.
    if config::get_current_project().is_some() {
        print_user_warning!("Workspace is already linked to a Phylum project");
        let should_continue = Confirm::new()
            .with_prompt("Overwrite existing project configuration?")
            .default(false)
            .interact()?;

        if !should_continue {
            return Ok(ExitCode::ProjectAlreadyInitialized.into());
        }
    }

    let cli_project = matches.get_one::<String>("project");
    let cli_group = matches.get_one::<String>("group");

    // Interactively prompt for missing information.
    let (project, group) = prompt(cli_project, cli_group)?;

    // Attempt to create the project.
    let response = project::create_project(api, &project, group.clone()).await?;

    // If project already exists, just link to it.
    match response {
        CommandValue::Code(ExitCode::AlreadyExists) => {
            project::link_project(api, &project, group).await
        },
        command_value => Ok(command_value),
    }
}

/// Interactively ask for missing information.
fn prompt(
    cli_project: Option<&String>,
    cli_group: Option<&String>,
) -> io::Result<(String, Option<String>)> {
    if let Some(project) = cli_project {
        return Ok((project.clone(), cli_group.cloned()));
    }

    // Prompt for project name.
    let project = prompt_project()?;

    // Prompt for group name.
    let group = match cli_group {
        Some(group) => Some(group.clone()),
        None => prompt_group()?,
    };

    Ok((project, group))
}

/// Ask for the desired project name.
fn prompt_project() -> io::Result<String> {
    // Use directory name as default project name.
    let current_dir = env::current_dir()?;
    let default_name = current_dir.file_name().and_then(|name| name.to_str());

    let theme = ColorfulTheme::default();
    let mut prompt = Input::with_theme(&theme);
    prompt.with_prompt("Project Name");

    if let Some(default_name) = default_name {
        prompt.default(default_name.to_owned());
    }

    prompt.interact_text()
}

// Ask for the desired group.
fn prompt_group() -> io::Result<Option<String>> {
    let should_prompt =
        Confirm::new().with_prompt("Use a project group?").default(false).interact()?;

    if !should_prompt {
        return Ok(None);
    }

    let group: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Project Group (default: none)")
        .interact_text()?;

    Ok(Some(group))
}
