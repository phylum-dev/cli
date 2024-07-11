//! Subcommand `phylum init`.

use std::path::Path;
use std::{env, io, iter};

use anyhow::Context;
use clap::parser::ValuesRef;
use clap::ArgMatches;
use dialoguer::{Confirm, FuzzySelect, Input, MultiSelect};
use git2::Repository;
use phylum_lockfile::LockfileFormat;
use phylum_project::{DepfileConfig, ProjectConfig, PROJ_CONF_FILE};
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{project, CommandResult, ExitCode};
use crate::types::UserGroup;
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
    let cli_depfile_type = matches.get_one::<String>("type");
    let cli_depfiles = matches.get_many::<String>("depfile");
    let cli_project = matches.get_one::<String>("project");
    let cli_group = matches.get_one::<String>("group");

    // Get available groups from API.
    let groups = api.get_groups_list().await?.groups;

    // Interactively prompt for missing project information.
    let (project, group, repository_url) =
        prompt_project(&groups, cli_project, cli_group, cli_repository_url).await?;

    // Interactively prompt for missing dependency file information.
    let depfiles = prompt_depfiles(cli_depfiles, cli_depfile_type)?;

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

    // Override project dependency file info.
    project_config.set_depfiles(depfiles);

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
        None => prompt_group(groups)?,
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
pub fn prompt_group(groups: &[UserGroup]) -> anyhow::Result<Option<String>> {
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

    // Suggest using the automatically inferred URL.
    if let Some(repository_url) = git_repository_url() {
        let use_inferred = Confirm::new()
            .with_prompt(format!("Use {repository_url:?} as repository URL?"))
            .default(true)
            .interact()?;

        if use_inferred {
            return Ok(Some(repository_url));
        }
    }

    // Prompt for selection of one group.
    let prompt = "[ENTER] Confirm\nRepository URL [default: None]";
    let input: String = Input::new().with_prompt(prompt).allow_empty(true).interact()?;

    println!();

    Ok((!input.is_empty()).then_some(input))
}

/// Interactively ask for missing dependency file information.
fn prompt_depfiles(
    cli_depfiles: Option<ValuesRef<'_, String>>,
    cli_depfile_type: Option<&String>,
) -> io::Result<Vec<DepfileConfig>> {
    // Prompt for dependency files if they weren't specified.
    let depfiles = match cli_depfiles {
        Some(depfiles) => depfiles.cloned().collect(),
        None => prompt_depfile_names()?,
    };

    // Find dependency file type for each file.
    let mut depfile_configs = Vec::new();
    for depfile in &depfiles {
        let depfile_type = find_depfile_type(depfile, cli_depfile_type)?;
        let config = DepfileConfig::new(depfile, depfile_type);
        depfile_configs.push(config);
    }

    Ok(depfile_configs)
}

/// Ask for the dependency file names.
fn prompt_depfile_names() -> io::Result<Vec<String>> {
    // Find all known dependency files below the currenty directory.
    let mut depfiles = phylum_lockfile::find_depfiles_at(".")
        .iter()
        .flat_map(|(path, _)| Some(path.to_str()?.to_owned()))
        .collect::<Vec<_>>();

    // Prompt for selection if any dependency file was found.
    let prompt = !depfiles.is_empty();
    if prompt {
        // Add choice to specify additional unidentified files.
        depfiles.push(String::from("others"));

        // Ask user for files.
        let indices = MultiSelect::new()
            .with_prompt("[SPACE] Select  [ENTER] Confirm\nSelect your project's dependency files")
            .items(&depfiles)
            .interact()?;

        // Remove unselected dependency files.
        let mut indices = indices.iter().peekable();
        let mut files_index = 0;
        depfiles.retain(|_| {
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
        match depfiles.last().map_or("others", |depfile| depfile.as_str()) {
            "others" => depfiles.pop(),
            _ => return Ok(depfiles),
        };
    }

    // Construct dialoguer freetext prompt.
    let mut input = Input::new();
    if prompt {
        input.with_prompt("Other dependency files (comma separated paths)");
    } else {
        input.with_prompt(
            "No known dependency files found in the current directory.\nDependency files (comma \
             separated paths)",
        );
    };

    // Allow empty as escape hatch if people already selected a valid dependency
    // file.
    input.allow_empty(!depfiles.is_empty());

    // Prompt for additional files.
    let other_depfiles: String = input.interact_text()?;

    println!();

    // Remove whitespace around files and add them to our list.
    for depfile in other_depfiles.split(',').map(|path| path.trim()).filter(|path| !path.is_empty())
    {
        depfiles.push(depfile.into());
    }

    Ok(depfiles)
}

/// Find the type for a dependency file path.
fn find_depfile_type(depfile: &str, cli_depfile_type: Option<&String>) -> io::Result<String> {
    if let Some(cli_depfile_type) = cli_depfile_type {
        // Use CLI dependency file type if specified.
        return Ok(cli_depfile_type.into());
    }

    // Find all matching lockfile types.
    let formats = LockfileFormat::iter().filter(|format| {
        let path = Path::new(&depfile);
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
    prompt_depfile_type(depfile, formats)
}

/// Ask for the dependency file type.
fn prompt_depfile_type(depfile: &str, mut formats: Vec<&str>) -> io::Result<String> {
    // Allow all formats if no matching formats were found.
    if formats.is_empty() {
        formats = LockfileFormat::iter().map(|format| format.name()).collect();
    }

    let prompt = format!("[ENTER] Select and Confirm\nSelect dependency file type for {depfile:?}");
    let index = FuzzySelect::new().with_prompt(prompt).items(&formats).interact()?;

    println!();

    Ok(formats[index].to_owned())
}

/// Get repository URL from current directory's git info.
fn git_repository_url() -> Option<String> {
    let repository = Repository::open(".").ok()?;
    let remote = repository.find_remote("origin").ok()?;
    let url = remote.url()?;
    url.starts_with("http").then(|| url.into())
}
