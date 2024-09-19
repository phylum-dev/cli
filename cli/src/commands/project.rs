use std::cmp::Ordering;
use std::fmt::Write;
use std::path::Path;

use anyhow::{anyhow, Context};
use clap::{ArgMatches, Command};
use console::style;
use dialoguer::{Confirm, FuzzySelect, Input};
use phylum_project::{ProjectConfig, PROJ_CONF_FILE};
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{init, CommandResult, ExitCode};
use crate::config::{self, Config};
use crate::format::Format;
use crate::{print, print_user_failure, print_user_success, print_user_warning};

/// Handle the project subcommand.
pub async fn handle_project(
    api: &PhylumApi,
    app: &mut Command,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    match matches.subcommand() {
        Some(("status", matches)) => handle_status(api, matches, config).await,
        Some(("create", matches)) => handle_create_project(api, matches, config).await,
        Some(("update", matches)) => handle_update_project(api, app, matches, config).await,
        Some(("delete", matches)) => handle_delete_project(api, matches, config).await,
        Some(("list", matches)) => handle_list_projects(api, matches).await,
        Some(("link", matches)) => handle_link_project(api, matches, config).await,
        _ => unreachable!("invalid clap configuration"),
    }
}

/// Create a Phylum project.
pub async fn create_project(
    api: &PhylumApi,
    project: &str,
    org: Option<String>,
    group: Option<String>,
    repository_url: Option<String>,
) -> Result<ProjectConfig, PhylumApiError> {
    let project_id = api.create_project(project, org, group.clone(), repository_url).await?;
    Ok(ProjectConfig::new(project_id.to_owned(), project.to_owned(), group))
}

/// Print project information.
async fn handle_status(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    let pretty_print = !matches.get_flag("json");
    let project = matches.get_one::<String>("project");
    let group = matches.get_one::<String>("group").map(|g| g.as_str());

    let project_id = match project {
        // If project is passed on CLI, lookup its ID.
        Some(project) => api.get_project_id(project, config.org(), group).await?,
        // If no project is passed, use `.phylum_project`.
        None => match phylum_project::get_current_project() {
            Some(project_config) => project_config.id,
            None => {
                if pretty_print {
                    print_user_success!("No project set");
                } else {
                    println!("{{}}");
                }

                return Ok(ExitCode::Ok);
            },
        },
    };

    let project = api.get_project(&project_id.to_string()).await?;

    project.write_stdout(pretty_print);

    Ok(ExitCode::Ok)
}

/// List the projects in this account.
async fn handle_list_projects(api: &PhylumApi, matches: &ArgMatches) -> CommandResult {
    let group = matches.get_one::<String>("group").map(|g| g.as_str());
    let org = matches.get_one::<String>("org").map(|o| o.as_str());
    let pretty_print = !matches.get_flag("json");
    let no_group = matches.get_flag("no-group");

    let mut resp = api.get_projects(org, group, None).await?;

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

    Ok(ExitCode::Ok)
}

/// Create a Phylum project.
async fn handle_create_project(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    let repository_url = matches.get_one::<String>("repository-url").cloned();
    let project = matches.get_one::<String>("name").unwrap();
    let group = matches.get_one::<String>("group").cloned();
    let org = config.org();

    log::info!("Initializing new project: `{}`", project);

    let project_config =
        create_project(api, project, org.map(|org| org.into()), group.clone(), repository_url)
            .await;
    let project_config = match project_config {
        Ok(project) => project,
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
            let formatted_project = format_project_reference(org, group.as_deref(), project, None);
            print_user_failure!("Project {} already exists", formatted_project);
            return Ok(ExitCode::AlreadyExists);
        },
        Err(err) => return Err(err.into()),
    };

    config::save_config(Path::new(PROJ_CONF_FILE), &project_config).unwrap_or_else(|err| {
        print_user_failure!("Failed to save project file: {}", err);
    });

    let project_id = Some(project_config.id.to_string());
    let formatted_project =
        format_project_reference(org, group.as_deref(), project, project_id.as_deref());
    print_user_success!("Successfully created project {formatted_project}");

    Ok(ExitCode::Ok)
}

/// Delete a Phylum project.
async fn handle_delete_project(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    let project_name = matches.get_one::<String>("name").unwrap();
    let group_name = matches.get_one::<String>("group").map(|g| g.as_str());
    let org = group_name.as_ref().and_then(|_| config.org());

    let formatted_project = format_project_reference(org, group_name, project_name, None);
    let proj_uuid = api
        .get_project_id(project_name, org, group_name)
        .await
        .with_context(|| format!("Project {formatted_project} does not exist"))?;

    api.delete_project(proj_uuid).await?;

    print_user_success!("Successfully deleted project {formatted_project}");

    Ok(ExitCode::Ok)
}

/// Update a Phylum project.
async fn handle_update_project(
    api: &PhylumApi,
    app: &mut Command,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    let repository_url_cli = matches.get_one::<String>("repository-url");
    let default_label_cli = matches.get_one::<String>("default-label");
    let project_id_cli = matches.get_one::<String>("project-id");
    let name_cli = matches.get_one::<String>("name");
    let org = config.org();

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
        return Ok(ExitCode::Ok);
    }

    // Prompt for project if necessary.
    let group_name = matches.get_one::<String>("group").cloned();
    let (project_id, group_name) = match project_id_cli {
        Some(project_id) => (project_id.clone(), group_name),
        None => prompt_project(api, org, group_name).await?,
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
        org.map(String::from),
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
    let formatted_project =
        format_project_reference(org, group_name.as_deref(), &project.name, Some(&project_id));
    let mut success_msg = format!("Successfully updated project {formatted_project}");
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

    Ok(ExitCode::Ok)
}

/// Link a Phylum project to the currenty directory.
async fn handle_link_project(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    let project_name = matches.get_one::<String>("name").unwrap();
    let group_name = matches.get_one::<String>("group").cloned();
    let org = group_name.as_ref().and_then(|_| config.org());

    let uuid = api.get_project_id(project_name, org, group_name.as_deref()).await?;

    let project_config = match phylum_project::get_current_project() {
        Some(mut project) => {
            project.update_project(uuid, project_name.into(), group_name.clone());
            project
        },
        None => ProjectConfig::new(uuid, project_name.into(), group_name.clone()),
    };

    config::save_config(Path::new(PROJ_CONF_FILE), &project_config)
        .unwrap_or_else(|err| log::error!("Failed to save user credentials to config: {}", err));

    let project_id = Some(project_config.id.to_string());
    let formatted_project =
        format_project_reference(org, group_name.as_deref(), project_name, project_id.as_deref());
    print_user_success!("Linked the current working directory to the project {formatted_project}.");

    Ok(ExitCode::Ok)
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
    org: Option<&str>,
    cli_group: Option<String>,
) -> anyhow::Result<(String, Option<String>)> {
    // Get the project group, prompting for it if necessary.
    let group_name = match cli_group {
        Some(cli_group) => Some(cli_group),
        None => {
            let groups: Vec<_> = match org {
                Some(org) => {
                    api.org_groups(org).await?.groups.into_iter().map(|group| group.name).collect()
                },
                None => {
                    let groups_list = api.get_groups_list().await?;
                    groups_list.groups.into_iter().map(|group| group.group_name).collect()
                },
            };
            init::prompt_group(&groups)?
        },
    };

    // Get all projects.
    let mut projects = api.get_projects(org, group_name.as_deref(), None).await?;

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

/// Format a project hierarchy for user output.
fn format_project_reference(
    org: Option<&str>,
    group: Option<&str>,
    project: &str,
    project_id: Option<&str>,
) -> String {
    let mut formatted = String::new();

    let _ = write!(formatted, "{}", style(project).green());

    formatted.push_str(" (");

    if let Some(project_id) = project_id {
        let _ = write!(formatted, "id: {}, ", style(project_id).green());
    }

    let _ = match group.and(org) {
        Some(org) => write!(formatted, "org: {}, ", style(org).green()),
        None => write!(formatted, "org: {}, ", style("-")),
    };

    let _ = match group {
        Some(group) => write!(formatted, "group: {}", style(group).green()),
        None => write!(formatted, "group: {}", style("-")),
    };

    formatted.push(')');

    formatted
}
