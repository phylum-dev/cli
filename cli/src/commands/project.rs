use std::path::Path;
use std::result::Result as StdResult;

use anyhow::{Context, Result};
use console::style;
use phylum_project::{ProjectConfig, PROJ_CONF_FILE};
use phylum_types::types::common::ProjectId;
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError, ResponseError};
use crate::commands::{CommandResult, ExitCode};
use crate::config::save_config;
use crate::format::Format;
use crate::{print_user_failure, print_user_success};

/// List the projects in this account.
pub async fn get_project_list(
    api: &mut PhylumApi,
    pretty_print: bool,
    group: Option<&str>,
) -> Result<()> {
    let resp = api.get_projects(group).await?;

    resp.write_stdout(pretty_print);

    Ok(())
}

/// Handle the project subcommand.
pub async fn handle_project(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        let name = matches.get_one::<String>("name").unwrap();
        let group = matches.get_one::<String>("group").cloned();

        log::info!("Initializing new project: `{}`", name);

        let project_config = match create_project(api, name, group).await {
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

/// Create and update the Phylum project.
pub async fn create_project(
    api: &PhylumApi,
    project: &str,
    group: Option<String>,
) -> StdResult<ProjectConfig, PhylumApiError> {
    let project_id = api.create_project(project, group.as_deref()).await?;

    Ok(ProjectConfig::new(project_id.to_owned(), project.to_owned(), group))
}

/// Lookup project by name and group.
pub async fn lookup_project(
    api: &PhylumApi,
    project: &str,
    group: Option<&str>,
) -> StdResult<ProjectId, PhylumApiError> {
    let uuid = api
        .get_project_id(project, group)
        .await
        .context("A project with that name does not exist")?;
    Ok(uuid)
}
