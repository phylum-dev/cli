use std::path::Path;
use std::result::Result as StdResult;

use anyhow::{anyhow, Context, Result};
use clap::{ArgMatches, Command};
use console::style;
use dialoguer::{Confirm, FuzzySelect, Input};
use phylum_project::{ProjectConfig, PROJ_CONF_FILE};
use phylum_types::types::common::ProjectId;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{init, CommandResult, ExitCode};
use crate::config::save_config;
use crate::format::Format;
use crate::{print, print_user_failure, print_user_success, print_user_warning};

/// List the projects in this account.
pub async fn get_project_list(
    api: &PhylumApi,
    pretty_print: bool,
    group: Option<&str>,
) -> Result<()> {
    let resp = api.get_projects(group).await?;

    resp.write_stdout(pretty_print);

    Ok(())
}

/// Handle the project subcommand.
pub async fn handle_project(
    api: &PhylumApi,
    app: &mut Command,
    matches: &ArgMatches,
) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        let name = matches.get_one::<String>("name").unwrap();
        let group = matches.get_one::<String>("group").cloned();
        let repository_url = matches.get_one::<String>("repository-url").cloned();

        log::info!("Initializing new project: `{}`", name);

        let project_config = match create_project(api, name, group, repository_url).await {
            Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
                print_user_failure!("Project '{}' already exists", name);
                return Ok(ExitCode::AlreadyExists);
            },
            project_config => project_config?,
        };

        save_config(Path::new(PROJ_CONF_FILE), &project_config).unwrap_or_else(|err| {
            print_user_failure!("Failed to save project file: {}", err);
        });

        print_user_success!("Successfully created new project, {}", name);
    } else if let Some(matches) = matches.subcommand_matches("update") {
        update_project(app, api, matches).await?;
    } else if let Some(matches) = matches.subcommand_matches("delete") {
        let project_name = matches.get_one::<String>("name").unwrap();
        let group_name = matches.get_one::<String>("group");

        let proj_uuid = api
            .get_project_id(project_name, group_name.map(String::as_str))
            .await
            .context("A project with that name does not exist")?;

        api.delete_project(proj_uuid).await?;

        print_user_success!("Successfully deleted project, {}", project_name);
    } else if let Some(matches) = matches.subcommand_matches("list") {
        let group = matches.get_one::<String>("group");
        let pretty_print = !matches.get_flag("json");
        get_project_list(api, pretty_print, group.map(String::as_str)).await?;
    } else if let Some(matches) = matches.subcommand_matches("link") {
        let project_name = matches.get_one::<String>("name").unwrap();
        let group_name = matches.get_one::<String>("group").cloned();

        let uuid = lookup_project(api, project_name, group_name.as_deref()).await?;

        let project_config = match phylum_project::get_current_project() {
            Some(mut project) => {
                project.update_project(uuid, project_name.into(), group_name);
                project
            },
            None => ProjectConfig::new(uuid, project_name.into(), group_name),
        };

        save_config(Path::new(PROJ_CONF_FILE), &project_config).unwrap_or_else(|err| {
            log::error!("Failed to save user credentials to config: {}", err)
        });

        print_user_success!(
            "Linked the current working directory to the project {}.",
            format!("{}", style(project_config.name).white())
        );
    }

    Ok(ExitCode::Ok)
}

/// Create a Phylum project.
pub async fn create_project(
    api: &PhylumApi,
    project: &str,
    group: Option<String>,
    repository_url: Option<String>,
) -> StdResult<ProjectConfig, PhylumApiError> {
    let project_id = api.create_project(project, group.clone(), repository_url).await?;

    Ok(ProjectConfig::new(project_id.to_owned(), project.to_owned(), group))
}

/// Lookup project by name and group.
pub async fn lookup_project(
    api: &PhylumApi,
    name: &str,
    group: Option<&str>,
) -> StdResult<ProjectId, PhylumApiError> {
    let uuid =
        api.get_project_id(name, group).await.context("A project with that name does not exist")?;
    Ok(uuid)
}

/// Update a Phylum project.
pub async fn update_project(
    app: &mut Command,
    api: &PhylumApi,
    matches: &ArgMatches,
) -> StdResult<(), PhylumApiError> {
    let repository_url_cli = matches.get_one::<String>("repository-url");
    let project_id_cli = matches.get_one::<String>("project-id");
    let name_cli = matches.get_one::<String>("name");

    // Determine interactivity by checking if project ID was supplied.
    let interactive = project_id_cli.is_none();

    // Sanity check non-interactive usage.
    if !interactive && repository_url_cli.is_none() && name_cli.is_none() {
        print_user_warning!("No changes requested, nothing to do.\n");
        print::print_sc_help(app, &["project", "update"])?;
        return Ok(());
    }

    // Prompt for project if necessary.
    let group_name = matches.get_one::<String>("group").cloned();
    let (project_id, group_name) = match project_id_cli {
        Some(project_id) => (project_id.clone(), group_name),
        None => prompt_project(api, group_name).await?,
    };

    // Get existing project information from the API.
    let project = api.get_project(&project_id, group_name.as_deref()).await?;

    // Check if repository URL should be changed.
    let change_repository_url = if interactive {
        let change_repository_url = Confirm::new()
            .with_prompt("Change repository URL?")
            .default(false)
            .interact()
            .map_err(|err| anyhow!(err))?;

        println!();

        change_repository_url
    } else {
        false
    };

    // Prompt for repository URL, defaulting to the existing one if empty.
    let repository_url = match repository_url_cli {
        None if change_repository_url => prompt_optional("New Repository URL", None)?,
        None => project.repository_url.clone(),
        Some(repository_url) => Some(repository_url.clone()),
    };

    // Prompt for name, defaulting to the existing one if empty.
    let name = match name_cli {
        None if interactive => {
            prompt_optional("New Project Name", Some(project.name.clone()))?.unwrap()
        },
        name => name.cloned().unwrap_or(project.name.clone()),
    };

    api.update_project(&project_id, group_name.clone(), name.clone(), repository_url.clone())
        .await?;

    // Output success message.
    let fmt_option = |opt| match opt {
        Some(s) => format!("{s:?}"),
        None => String::from("None"),
    };
    let mut success_msg = format!("Successfully updated project {project_id:?}");
    if let Some(group_name) = &group_name {
        success_msg += &format!(" (group {group_name:?})");
    }
    success_msg += ":\n";
    success_msg += &format!("      Name: {:?} -> {name:?}\n", project.name);
    success_msg += &format!(
        "      Repository URL: {} -> {}",
        fmt_option(project.repository_url),
        fmt_option(repository_url)
    );
    print_user_success!("{}", success_msg);

    Ok(())
}

/// Prompt for optional text input.
fn prompt_optional(subject: &str, default: Option<String>) -> anyhow::Result<Option<String>> {
    // Prompt for selection of one group.
    let prompt = match &default {
        Some(default) => format!("[ENTER] Confirm\n{subject} [default: {default}]"),
        None => format!("[ENTER] Confirm\n{subject} [default: None]"),
    };
    let input: String = Input::new().with_prompt(prompt).allow_empty(true).interact()?;

    println!();

    if input.is_empty() {
        Ok(default)
    } else {
        Ok(Some(input))
    }
}

/// Prompt for project selection.
async fn prompt_project(
    api: &PhylumApi,
    cli_group: Option<String>,
) -> anyhow::Result<(String, Option<String>)> {
    // Get the project group, prompting for it if necessary.
    let group_name = match cli_group {
        Some(cli_group) => Some(cli_group),
        None => {
            let groups = api.get_groups_list().await?.groups;
            init::prompt_group(&groups)?
        },
    };

    // Get all projects.
    let projects = api.get_projects(group_name.as_deref()).await?;
    let project_names: Vec<_> = projects.iter().map(|project| &project.name).collect();

    // Prompt for project selection.
    let prompt = "[ENTER] Confirm\nProject Name";
    let index = FuzzySelect::new().with_prompt(prompt).items(&project_names).interact()?;
    let project_id = projects[index].id.to_string();

    println!();

    Ok((project_id, group_name))
}
