use std::cmp::Ordering;
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
    no_group: bool,
) -> Result<()> {
    let mut resp = api.get_projects(group, None).await?;

    // Remove group projects if requested.
    if no_group {
        resp.retain(|project| project.group_name.is_none());
    }

    // Sort response for nicer output.
    resp.sort_unstable_by(|a, b| match a.group_name.cmp(&b.group_name) {
        Ordering::Equal => a.name.cmp(&b.name),
        ordering => ordering,
    });

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

        print_user_success!("Successfully created project {name:?} ({})", project_config.id);
    } else if let Some(matches) = matches.subcommand_matches("status") {
        status(api, matches).await?;
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
        let no_group = matches.get_flag("no-group");
        get_project_list(api, pretty_print, group.map(String::as_str), no_group).await?;
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

/// Print project information.
pub async fn status(api: &PhylumApi, matches: &ArgMatches) -> StdResult<(), PhylumApiError> {
    let pretty_print = !matches.get_flag("json");
    let project = matches.get_one::<String>("project");
    let group = matches.get_one::<String>("group");

    let project_id = match project {
        // If project is passed on CLI, lookup its ID.
        Some(project) => lookup_project(api, project, group.map(|g| g.as_str())).await?,
        // If no project is passed, use `.phylum_project`.
        None => match phylum_project::get_current_project() {
            Some(project_config) => project_config.id,
            None => {
                if pretty_print {
                    print_user_success!("No project set");
                } else {
                    println!("{{}}");
                }

                return Ok(());
            },
        },
    };

    let project = api.get_project(&project_id.to_string()).await?;

    project.write_stdout(pretty_print);

    Ok(())
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
    let default_label_cli = matches.get_one::<String>("default-label");
    let project_id_cli = matches.get_one::<String>("project-id");
    let name_cli = matches.get_one::<String>("name");

    // Determine interactivity by checking if project ID was supplied.
    let interactive = project_id_cli.is_none();

    // Sanity check non-interactive usage.
    if !interactive
        && name_cli.is_none()
        && repository_url_cli.is_none()
        && default_label_cli.is_none()
    {
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
    let project = api.get_project(&project_id).await?;

    // Prompt for name, defaulting to the existing one if empty.
    let name = match name_cli {
        None if interactive => {
            prompt_optional("New Project Name", Some(project.name.clone()))?.unwrap()
        },
        name => name.cloned().unwrap_or(project.name.clone()),
    };

    // Check if repository URL should be changed.
    let change_repository_url = if interactive && repository_url_cli.is_none() {
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

    // Check if default label should be changed.
    let change_default_label = if interactive && default_label_cli.is_none() {
        let change_default_label = Confirm::new()
            .with_prompt("Change default label?")
            .default(false)
            .interact()
            .map_err(|err| anyhow!(err))?;

        println!();

        change_default_label
    } else {
        false
    };

    // Prompt for default label if necessary.
    let default_label = match default_label_cli {
        None if change_default_label => prompt_optional("New Default Label", None)?,
        None => project.default_label.clone(),
        Some(default_label) => Some(default_label.clone()),
    };

    api.update_project(
        &project_id,
        group_name.clone(),
        name.clone(),
        repository_url.clone(),
        default_label.clone(),
    )
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
        "      Repository URL: {} -> {}\n",
        fmt_option(project.repository_url),
        fmt_option(repository_url)
    );
    success_msg += &format!(
        "      Default Label: {} -> {}",
        fmt_option(project.default_label),
        fmt_option(default_label),
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
    let mut projects = api.get_projects(group_name.as_deref(), None).await?;

    // Remove group projects if the user didn't select any group.
    projects.retain(|project| project.group_name.is_some() == group_name.is_some());

    let project_names: Vec<_> = projects.iter().map(|project| &project.name).collect();

    // Prompt for project selection.
    let prompt = "[ENTER] Confirm\nProject Name";
    let index = FuzzySelect::new().with_prompt(prompt).items(&project_names).interact()?;
    let project_id = projects[index].id.to_string();

    println!();

    Ok((project_id, group_name))
}
