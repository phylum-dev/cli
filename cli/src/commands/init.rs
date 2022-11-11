//! Subcommand `phylum init`.

use std::path::Path;
use std::{env, fs, io};

use anyhow::Context;
use clap::ArgMatches;
use dialoguer::{Confirm, FuzzySelect, Input};
use phylum_lockfile::LockfileFormat;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{project, CommandResult, ExitCode};
use crate::config::PROJ_CONF_FILE;
use crate::{config, print_user_failure, print_user_success, print_user_warning};

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

        println!();
    }

    let cli_project = matches.get_one::<String>("project");
    let cli_group = matches.get_one::<String>("group");

    // Interactively prompt for missing information.
    let (project, group) = prompt_project(cli_project, cli_group)?;

    // Attempt to create the project.
    let result = project::create_project(api, &project, group.clone()).await;

    // If project already exists, just link to it.
    let mut project_config = match result {
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            print_user_success!("Successfully linked to project {project:?}");
            project::link_project(api, &project, group).await.context("Unable to link project")?
        },
        project_config => {
            if project_config.is_ok() {
                print_user_success!("Successfully created project {project:?}");
            }
            project_config.context("Unable to create project")?
        },
    };

    let cli_lockfile_type = matches.get_one::<String>("lockfile-type");
    let cli_lockfile = matches.get_one::<String>("lockfile");

    // Add lockfile and its type to the project configuration.
    println!();
    let (lockfile, lockfile_type) = prompt_lockfile(cli_lockfile, cli_lockfile_type)?;
    project_config.lockfile_type = lockfile_type;
    project_config.lockfile = Some(lockfile);

    // Save project config.
    config::save_config(Path::new(PROJ_CONF_FILE), &project_config).unwrap_or_else(|err| {
        print_user_failure!("Failed to save project file: {}", err);
    });

    Ok(ExitCode::Ok.into())
}

/// Interactively ask for missing project information.
fn prompt_project(
    cli_project: Option<&String>,
    cli_group: Option<&String>,
) -> io::Result<(String, Option<String>)> {
    if let Some(project) = cli_project {
        return Ok((project.clone(), cli_group.cloned()));
    }

    // Prompt for project name.
    let project = prompt_project_name()?;

    // Prompt for group name.
    let group = match cli_group {
        Some(group) => Some(group.clone()),
        None => prompt_group()?,
    };

    Ok((project, group))
}

/// Ask for the desired project name.
fn prompt_project_name() -> io::Result<String> {
    // Use directory name as default project name.
    let current_dir = env::current_dir()?;
    let default_name = current_dir.file_name().and_then(|name| name.to_str());

    let mut prompt = Input::new();
    prompt.with_prompt("Project Name");

    if let Some(default_name) = default_name {
        prompt.default(default_name.to_owned());
    }

    prompt.interact_text()
}

/// Ask for the desired group.
fn prompt_group() -> io::Result<Option<String>> {
    let should_prompt =
        Confirm::new().with_prompt("Use a project group?").default(false).interact()?;

    if !should_prompt {
        return Ok(None);
    }

    let group: String =
        Input::new().with_prompt("Project Group (default: none)").interact_text()?;

    Ok(Some(group))
}

/// Interactively ask for missing lockfile information.
fn prompt_lockfile(
    cli_lockfile: Option<&String>,
    cli_lockfile_type: Option<&String>,
) -> io::Result<(String, Option<String>)> {
    if let Some(lockfile) = cli_lockfile {
        return Ok((lockfile.clone(), cli_lockfile_type.cloned()));
    }

    // Prompt for lockfile name.
    let lockfile = prompt_lockfile_name()?;

    // Try to determine lockfile name from known lockfiles.
    for format in LockfileFormat::iter() {
        if format.parser().is_path_lockfile(Path::new(&lockfile)) {
            let lockfile_type = format.name().to_owned();
            return Ok((lockfile, Some(lockfile_type)));
        }
    }

    // Prompt for lockfile type.
    let lockfile_type = prompt_lockfile_type()?;

    Ok((lockfile, Some(lockfile_type)))
}

/// Ask for the lockfile name.
fn prompt_lockfile_name() -> io::Result<String> {
    // Find all known lockfiles in the currenty directory.
    let mut lockfiles: Vec<_> = fs::read_dir("./")?
        .into_iter()
        .flatten()
        .filter(|entry| {
            LockfileFormat::iter().any(|format| format.parser().is_path_lockfile(&entry.path()))
        })
        .flat_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect();

    // Prompt if a lockfile was found.
    if !lockfiles.is_empty() {
        // Add choice to specify an unknown lockfile.
        lockfiles.push(String::from("other"));

        // Ask user for lockfile.
        let index = FuzzySelect::new()
            .with_prompt("Select your project's lockfile")
            .items(&lockfiles)
            .interact()?;

        // Return selected lockfile unless `other` was chosen.
        if index + 1 != lockfiles.len() {
            return Ok(lockfiles.remove(index));
        }
    }

    // Prompt for lockfile name if none was selected.
    Input::new().with_prompt("Project lockfile name").interact_text()
}

/// Ask for the lockfile type.
fn prompt_lockfile_type() -> io::Result<String> {
    let lockfile_types: Vec<_> = LockfileFormat::iter().map(|format| format.name()).collect();

    let index = FuzzySelect::new()
        .with_prompt("Select lockfile's type")
        .items(&lockfile_types)
        .interact()?;

    Ok(lockfile_types[index].to_owned())
}
