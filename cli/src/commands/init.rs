//! Subcommand `phylum init`.

use std::{io, env};

use clap::ArgMatches;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};

use crate::api::PhylumApi;
use crate::commands::{project, CommandResult, CommandValue, ExitCode};

/// Handle `phylum init` subcommand.
pub async fn handle_init(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    let cli_project = matches.get_one::<String>("project");
    let cli_group = matches.get_one::<String>("group");

    let (project, group) = match cli_project {
        Some(project) => (project.clone(), cli_group.cloned()),
        None => {
            let project = prompt_project()?;
            let group = match cli_group {
                Some(group) => Some(group.clone()),
                None => prompt_group()?,
            };
            (project, group)
        },
    };

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
