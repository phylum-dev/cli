use std::path::Path;

use ansi_term::Color::{Blue, White};
use anyhow::{anyhow, Context};
use chrono::Local;

use super::{CommandResult, ExitCode};
use crate::api::PhylumApi;
use crate::config::{get_current_project, save_config, ProjectConfig, PROJ_CONF_FILE};
use crate::print::*;
use crate::print_user_failure;
use crate::print_user_success;
use crate::prompt::prompt_threshold;

/// List the projects in this account.
pub async fn get_project_list(api: &mut PhylumApi, pretty_print: bool, group: Option<String>) {
    let resp = api.get_projects(group).await;

    // Print table header when we're not outputting in JSON format.
    if pretty_print {
        let proj_title = format!("{}", Blue.paint("Project Name"));
        let id_title = format!("{}", Blue.paint("Project ID"));
        println!("{:<38}{}", proj_title, id_title);
    }

    print_response(&resp, pretty_print, None);
    println!();
}

/// Handle the project subcommand. Provides facilities for creating a new project,
/// linking a current repository to an existing project, listing projects and
/// setting project thresholds for risk domains.
pub async fn handle_project(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let pretty_print = !matches.is_present("json");

    if let Some(matches) = matches.subcommand_matches("create") {
        let name = matches.value_of("name").unwrap();
        let group = matches.value_of("group");

        log::info!("Initializing new project: `{}`", name);

        let project_id = api.create_project(name, group).await?;

        let proj_conf = ProjectConfig {
            id: project_id.to_owned(),
            created_at: Local::now(),
            group_name: group.map(String::from),
            name: name.to_owned(),
        };

        save_config(Path::new(PROJ_CONF_FILE), &proj_conf).unwrap_or_else(|err| {
            print_user_failure!("Failed to save project file: {}", err);
        });

        print_user_success!("Successfully created new project, {}", name);
    } else if let Some(matches) = matches.subcommand_matches("list") {
        let group = matches.value_of("group").map(String::from);
        let pretty_print = pretty_print && !matches.is_present("json");
        get_project_list(api, pretty_print, group).await;
    } else if let Some(matches) = matches.subcommand_matches("link") {
        let project_name = matches.value_of("name").unwrap();
        let group_name = matches.value_of("group");

        let proj_uuid = api
            .get_project_id(project_name)
            .await
            .context("A project with that name does not exist")?;

        let proj_conf = ProjectConfig {
            id: proj_uuid,
            name: project_name.into(),
            created_at: Local::now(),
            group_name: group_name.map(String::from),
        };
        save_config(Path::new(PROJ_CONF_FILE), &proj_conf).unwrap_or_else(|err| {
            log::error!("Failed to save user credentials to config: {}", err)
        });

        print_user_success!(
            "Linked the current working directory to the project {}.",
            format!("{}", White.paint(proj_conf.name))
        );
    } else if let Some(matches) = matches.subcommand_matches("set-thresholds") {
        let mut project_name = matches.value_of("name").unwrap_or("current");

        let proj = if project_name == "current" {
            get_current_project().map(|p| p.name)
        } else {
            None
        };

        project_name = proj.as_deref().unwrap_or(project_name);
        log::debug!("Setting thresholds for project `{}`", project_name);

        println!("Risk thresholds allow you to specify what constitutes a failure.");
        println!("You can set a threshold for the overall project score, or for individual");
        println!("risk vectors:");
        println!();
        println!("    * Author");
        println!("    * Malicious Code");
        println!("    * Vulnerability");
        println!("    * License");
        println!("    * Engineering");
        println!();
        println!("If your project score falls below a given threshold, it will be");
        println!("considered a failure and the action you specify will be taken.");
        println!();
        println!("Possible actions are:");
        println!();
        println!(
            "    * {}: print a message to standard error",
            format_args!("{}", White.paint("Print a warning"))
        );
        println!(
            "    * {}: If we are in CI/CD break the build and return a non-zero exit code",
            format_args!("{}", White.paint("Break the build"))
        );
        println!(
            "    * {}: Ignore the failure and continue",
            format_args!("{}", White.paint("Nothing, fail silently"))
        );
        println!();

        println!(
            "Specify the thresholds and actions for {}. Accepted values are 0-100 or 'Disabled'.",
            format_args!("{}", White.paint(project_name))
        );
        println!();

        let project_id = api
            .get_project_id(project_name)
            .await
            .context("Could not get project ID")?;

        let mut user_settings = match api.get_user_settings().await {
            Ok(x) => x,
            Err(e) => {
                log::error!("Failed to get user settings: {}", e);
                return Err(anyhow!("Could not get user settings"));
            }
        };

        for threshold_name in &[
            "total project",
            "author",
            "engineering",
            "license",
            "malicious code",
            "vulnerability",
        ] {
            let threshold = match prompt_threshold(threshold_name) {
                Ok(threshold) => threshold,
                Err(_) => {
                    print_user_failure!("Failed to read user input");
                    continue;
                }
            };

            // API expects slight key change for specific fields.
            let name = match *threshold_name {
                "total project" => String::from("total"),
                "malicious code" => String::from("maliciousCode"),
                x => x.to_string(),
            };

            user_settings.set_threshold(project_id.to_string(), name, threshold);
        }

        let resp = api.put_user_settings(&user_settings).await;
        match resp {
            Ok(_) => {
                print_user_success!(
                    "Set all thresholds for the {} project",
                    White.paint(project_name)
                );
            }
            _ => {
                print_user_failure!(
                    "Failed to set thresholds for the {} project",
                    White.paint(project_name)
                );
                return Ok(ExitCode::SetThresholdsFailure.into());
            }
        }
    } else {
        let group = matches.value_of("group").map(String::from);
        get_project_list(api, pretty_print, group).await;
    }

    Ok(ExitCode::Ok.into())
}
