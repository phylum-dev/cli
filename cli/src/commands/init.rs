//! Subcommand `phylum init`.

use std::path::Path;
use std::{env, io, iter};

use anyhow::Context;
use clap::parser::ValuesRef;
use clap::ArgMatches;
use dialoguer::{Confirm, FuzzySelect, Input, MultiSelect};
use phylum_lockfile::LockfileFormat;
use phylum_project::{LockfileConfig, ProjectConfig, PROJ_CONF_FILE};
use phylum_types::types::group::UserGroup;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{project, CommandResult, ExitCode};
use crate::{config, print_user_success, print_user_warning};

/// Handle `phylum init` subcommand.
pub async fn handle_init(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    // Prompt for confirmation if a linked project is already in this directory.
    if !matches.get_flag("force") && phylum_project::find_project_conf(".", false).is_some() {
        print_user_warning!("Workspace is already linked to a Phylum project");
        let should_continue = Confirm::new()
            .with_prompt("Overwrite existing project configuration?")
            .default(false)
            .interact()?;

        if !should_continue {
            return Ok(ExitCode::ConfirmationFailed);
        }

        println!();
    }

    let cli_repository_url = matches.get_one::<String>("repository-url");
    let cli_lockfile_type = matches.get_one::<String>("lockfile-type");
    let cli_lockfiles = matches.get_many::<String>("lockfile");
    let cli_project = matches.get_one::<String>("project");
    let cli_group = matches.get_one::<String>("group");

    // Get available groups from API.
    let groups = api.get_groups_list().await?.groups;

    // Interactively prompt for missing project information.
    let (project, group, repository_url) =
        prompt_project(&groups, cli_project, cli_group, cli_repository_url).await?;

    // Interactively prompt for missing lockfile information.
    let lockfiles = prompt_lockable_files(cli_lockfiles, cli_lockfile_type)?;

    // Attempt to create the project.
    let result = project::create_project(api, &project, group.clone(), repository_url).await;

    let mut project_config = match result {
        // If project already exists, try looking it up to link to it.
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            let uuid = project::lookup_project(api, &project, group.as_deref())
                .await
                .context(format!("Could not find project {project:?}"))?;
            ProjectConfig::new(uuid, project, group)
        },
        project_config => project_config.context("Unable to create project")?,
    };

    // Override project lockfile info.
    project_config.set_lockfiles(lockfiles);

    // Save project config.
    config::save_config(Path::new(PROJ_CONF_FILE), &project_config)
        .context("Failed to save project file")?;

    print_user_success!("Successfully created project configuration");

    Ok(ExitCode::Ok)
}

/// Interactively ask for missing project information.
async fn prompt_project(
    groups: &[UserGroup],
    cli_project: Option<&String>,
    cli_group: Option<&String>,
    cli_repository_url: Option<&String>,
) -> anyhow::Result<(String, Option<String>, Option<String>)> {
    if let Some(project) = cli_project {
        return Ok((project.clone(), cli_group.cloned(), cli_repository_url.cloned()));
    }

    // Prompt for project name.
    let project = prompt_project_name()?;

    // Prompt for group name.
    let group = match cli_group {
        Some(group) => Some(group.clone()),
        None => prompt_group(groups).await?,
    };

    // Prompt for repository URL.
    let repository_url = match cli_repository_url {
        Some(repository_url) => Some(repository_url.clone()),
        None => prompt_repository_url().await?,
    };

    Ok((project, group, repository_url))
}

/// Ask for the desired project name.
fn prompt_project_name() -> io::Result<String> {
    // Use directory name as default project name.
    let current_dir = env::current_dir()?;
    let default_name = current_dir.file_name().and_then(|name| name.to_str());

    let mut prompt = Input::new();

    // Suggest default if we found a directory name.
    //
    // NOTE: We don't use dialoguer's built-in default here so we can add a more
    // explicit `default` label.
    match default_name {
        Some(default_name) => {
            prompt.with_prompt(format!("Project Name [default: {default_name}]"));
            prompt.allow_empty(true);
        },
        None => {
            let _ = prompt.with_prompt("Project Name");
        },
    };

    let mut name: String = prompt.interact_text()?;

    // Fallback to project name for empty strings.
    if name.is_empty() {
        name = default_name.expect("illegal empty project name").into();
    }

    Ok(name)
}

/// Ask for the desired group.
async fn prompt_group(groups: &[UserGroup]) -> anyhow::Result<Option<String>> {
    // Skip group selection if user has none.
    if groups.is_empty() {
        return Ok(None);
    }

    // Map groups to their name.
    let group_names = iter::once("[None]")
        .chain(groups.iter().map(|group| group.group_name.as_str()))
        .collect::<Vec<_>>();

    println!();

    // Prompt for selection of one group.
    let prompt = "[ENTER] Confirm\nProject Group";
    let index = FuzzySelect::new().with_prompt(prompt).items(&group_names).default(0).interact()?;

    println!();

    match index {
        0 => Ok(None),
        index => Ok(group_names.get(index).cloned().map(String::from)),
    }
}

/// Ask for the desired repository URL.
async fn prompt_repository_url() -> anyhow::Result<Option<String>> {
    println!();

    // Prompt for selection of one group.
    let prompt = "[ENTER] Confirm\nRepository URL [default: None]";
    let input: String = Input::new().with_prompt(prompt).allow_empty(true).interact()?;

    println!();

    Ok((!input.is_empty()).then_some(input))
}

/// Interactively ask for missing lockfile or manifest information.
fn prompt_lockable_files(
    cli_lockfiles: Option<ValuesRef<'_, String>>,
    cli_lockfile_type: Option<&String>,
) -> io::Result<Vec<LockfileConfig>> {
    // Prompt for lockfiles or manifests if they weren't specified.
    let lockfiles = match cli_lockfiles {
        Some(lockfiles) => lockfiles.cloned().collect(),
        None => prompt_lockable_file_names()?,
    };

    // Find lockfile type for each file.
    let mut lockfile_configs = Vec::new();
    for lockfile in &lockfiles {
        let lockfile_type = find_lockfile_type(lockfile, cli_lockfile_type)?;
        let config = LockfileConfig::new(lockfile, lockfile_type);
        lockfile_configs.push(config);
    }

    Ok(lockfile_configs)
}

/// Ask for the lockfile and manifest names.
fn prompt_lockable_file_names() -> io::Result<Vec<String>> {
    // Find all known lockfiles and manifests below the currenty directory.
    let mut lockable_files = phylum_lockfile::find_lockable_files_at(".")
        .iter()
        .flat_map(|(path, _)| Some(path.to_str()?.to_owned()))
        .collect::<Vec<_>>();

    // Prompt for selection if any lockfile was found.
    let prompt = !lockable_files.is_empty();
    if prompt {
        // Add choice to specify additional unidentified files.
        lockable_files.push(String::from("others"));

        // Ask user for files.
        let indices = MultiSelect::new()
            .with_prompt(
                "[SPACE] Select  [ENTER] Confirm\nSelect your project's lockfiles and manifests",
            )
            .items(&lockable_files)
            .interact()?;

        // Remove unselected lockfiles.
        let mut indices = indices.iter().peekable();
        let mut files_index = 0;
        lockable_files.retain(|_| {
            // Check if index is in the selected indices.
            let retain = indices.peek().map_or(false, |index| **index <= files_index);

            // Go to next selection index if current index was found.
            if retain {
                indices.next();
            }

            files_index += 1;

            retain
        });

        println!();

        // Return files if we found at least one and no others were requested.
        match lockable_files.last().map_or("others", |lockfile| lockfile.as_str()) {
            "others" => lockable_files.pop(),
            _ => return Ok(lockable_files),
        };
    }

    // Construct dialoguer freetext prompt.
    let mut input = Input::new();
    if prompt {
        input.with_prompt("Other lockfiles or manifests (comma separated paths)");
    } else {
        input.with_prompt(
            "No known lockfiles or manifests found in the current directory.\nLockfiles or \
             manifests (comma separated paths)",
        );
    };

    // Allow empty as escape hatch if people already selected a valid lockfile.
    input.allow_empty(!lockable_files.is_empty());

    // Prompt for additional files.
    let other_lockable_files: String = input.interact_text()?;

    println!();

    // Remove whitespace around files and add them to our list.
    for lockfile in
        other_lockable_files.split(',').map(|path| path.trim()).filter(|path| !path.is_empty())
    {
        lockable_files.push(lockfile.into());
    }

    Ok(lockable_files)
}

/// Find lockfile type for a lockable file path.
fn find_lockfile_type(lockfile: &str, cli_lockfile_type: Option<&String>) -> io::Result<String> {
    if let Some(cli_lockfile_type) = cli_lockfile_type {
        // Use CLI lockfile type if specified.
        return Ok(cli_lockfile_type.into());
    }

    // Find all matching lockfile types.
    let formats = LockfileFormat::iter().filter(|format| {
        let path = Path::new(&lockfile);
        let parser = format.parser();
        parser.is_path_lockfile(path) || parser.is_path_manifest(path)
    });
    let formats = formats.map(|format| format.name()).collect::<Vec<_>>();

    // Pick format if only one is available.
    if formats.len() == 1 {
        let lockfile_type = formats[0].to_owned();
        return Ok(lockfile_type);
    }

    // Prompt if multiple or no formats were detected.
    prompt_lockfile_type(lockfile, formats)
}

/// Ask for the lockfile type.
fn prompt_lockfile_type(lockfile: &str, mut formats: Vec<&str>) -> io::Result<String> {
    // Allow all formats if no matching formats were found.
    if formats.is_empty() {
        formats = LockfileFormat::iter().map(|format| format.name()).collect();
    }

    let prompt = format!("[ENTER] Select and Confirm\nSelect lockfile type for {lockfile:?}");
    let index = FuzzySelect::new().with_prompt(prompt).items(&formats).interact()?;

    println!();

    Ok(formats[index].to_owned())
}
