//! Subcommand `phylum init`.

use std::path::Path;
use std::{env, fs, io};

use anyhow::Context;
use clap::ArgMatches;
use dialoguer::{Confirm, FuzzySelect, Input, MultiSelect};
use phylum_lockfile::LockfileFormat;
use phylum_project::{LockfileConfig, PROJ_CONF_FILE};
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{project, CommandResult, ExitCode};
use crate::{config, print_user_success, print_user_warning};

/// Handle `phylum init` subcommand.
pub async fn handle_init(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    // Prompt for confirmation if a linked project is already in this directory.
    if !matches.get_flag("force") && phylum_project::find_project_conf(".", false).is_some() {
        print_user_warning!("Workspace is already linked to a Phylum project");
        let should_continue = Confirm::new()
            .with_prompt("Overwrite existing project configuration?")
            .default(false)
            .interact()?;

        if !should_continue {
            return Ok(ExitCode::ConfirmationFailed.into());
        }

        println!();
    }

    let cli_lockfile_type = matches.get_one::<String>("lockfile-type");
    let cli_lockfile = matches.get_one::<String>("lockfile");
    let cli_project = matches.get_one::<String>("project");
    let cli_group = matches.get_one::<String>("group");

    // Interactively prompt for missing project information.
    let (project, group) = prompt_project(cli_project, cli_group)?;

    // Interactively prompt for missing lockfile information.
    println!();
    let lockfiles = prompt_lockfile(cli_lockfile, cli_lockfile_type)?;

    // Attempt to create the project.
    println!();
    let result = project::create_project(api, &project, group.clone()).await;

    let mut project_config = match result {
        // If project already exists, try looking it up to link to it.
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            project::lookup_project(api, &project, group)
                .await
                .context(format!("Could not find project {project:?}"))?
        },
        project_config => project_config.context("Unable to create project")?,
    };

    // Override project lockfile info.
    project_config.set_lockfiles(lockfiles);

    // Save project config.
    config::save_config(Path::new(PROJ_CONF_FILE), &project_config)
        .context("Failed to save project file")?;

    print_user_success!("Successfully created project configuration");

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
    let group: String = Input::new()
        .with_prompt("Project Group [default: no group]")
        .allow_empty(true)
        .interact_text()?;

    Ok((!group.is_empty()).then_some(group))
}

/// Interactively ask for missing lockfile information.
fn prompt_lockfile(
    cli_lockfile: Option<&String>,
    cli_lockfile_type: Option<&String>,
) -> io::Result<Vec<LockfileConfig>> {
    let lockfiles = match (cli_lockfile.cloned(), cli_lockfile_type) {
        // Do not prompt if name and type were specified on CLI.
        (Some(lockfile), Some(lockfile_type)) => {
            let lockfiles = vec![LockfileConfig::new(lockfile, lockfile_type.into())];
            return Ok(lockfiles);
        },
        // Prompt for type if only lockfile was passed.
        (Some(lockfile), _) => vec![lockfile],
        // Prompt for lockfiles if it wasn't specified.
        (None, _) => prompt_lockfile_names()?,
    };

    // Find lockfile type for each lockfile.
    let mut lockfile_configs = Vec::new();
    for lockfile in &lockfiles {
        // Try to determine lockfile type from known formats.
        if let Some(format) = LockfileFormat::iter()
            .find(|format| format.parser().is_path_lockfile(Path::new(&lockfile)))
        {
            let lockfile_type = format.name().to_owned();
            lockfile_configs.push(LockfileConfig::new(lockfile, lockfile_type));
            continue;
        }

        // Prompt for lockfile type.
        let lockfile_type = prompt_lockfile_type(lockfile)?;

        lockfile_configs.push(LockfileConfig::new(lockfile, lockfile_type));
    }

    Ok(lockfile_configs)
}

/// Ask for the lockfile names.
fn prompt_lockfile_names() -> io::Result<Vec<String>> {
    // Find all known lockfiles in the currenty directory.
    let mut lockfiles: Vec<_> = fs::read_dir("./")?
        .flatten()
        .filter(|entry| {
            LockfileFormat::iter().any(|format| format.parser().is_path_lockfile(&entry.path()))
        })
        .flat_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect();

    // Prompt for selection if any lockfile was found.
    if !lockfiles.is_empty() {
        // Add choice to specify additional unidentified lockfiles.
        lockfiles.push(String::from("others"));

        // Ask user for lockfiles.
        let indices = MultiSelect::new()
            .with_prompt("Select your project's lockfile")
            .items(&lockfiles)
            .interact()?;

        // Remove unselected lockfiles.
        let mut indices = indices.iter().peekable();
        let mut lockfiles_index = 0;
        lockfiles.retain(|_| {
            // Check if lockfile index is in the selected indices.
            let retain = indices.peek().map_or(false, |index| **index <= lockfiles_index);

            // Go to next selection index if current index was found.
            if retain {
                indices.next();
            }

            lockfiles_index += 1;

            retain
        });

        // Return lockfiles if we found at least one and no others were requested.
        match lockfiles.last().map_or("others", |lockfile| lockfile.as_str()) {
            "others" => lockfiles.pop(),
            _ => return Ok(lockfiles),
        };
    }

    // Prompt for additional lockfiles.
    let prompt = "Other lockfiles (comma separated)";
    let other_lockfiles: String = Input::new().with_prompt(prompt).interact_text()?;

    // Remove whitespace around lockfiles and add them to our list.
    for lockfile in other_lockfiles.split(',') {
        lockfiles.push(lockfile.trim().into());
    }

    Ok(lockfiles)
}

/// Ask for the lockfile type.
fn prompt_lockfile_type(lockfile: &str) -> io::Result<String> {
    let lockfile_types: Vec<_> = LockfileFormat::iter().map(|format| format.name()).collect();

    let prompt = format!("Select type for lockfile {lockfile:?}");
    let index = FuzzySelect::new().with_prompt(prompt).items(&lockfile_types).interact()?;

    Ok(lockfile_types[index].to_owned())
}
