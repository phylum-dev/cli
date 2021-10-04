use ansi_term::Color::{Blue, White};

use chrono::Local;
use phylum_cli::api::PhylumApi;
use phylum_cli::config::{get_current_project, save_config, ProjectConfig};
use phylum_cli::types::PROJ_CONF_FILE;
use uuid::Uuid;

use crate::exit::exit_error;
use crate::print::*;

use crate::print_user_failure;
use crate::print_user_success;
use crate::prompt::prompt_threshold;

/// List the projects in this account.
pub async fn get_project_list(api: &mut PhylumApi, pretty_print: bool) {
    let resp = api.get_projects().await;
    let proj_title = format!("{}", Blue.paint("Project Name"));
    let id_title = format!("{}", Blue.paint("Project ID"));
    println!("{:<38}{}", proj_title, id_title);
    print_response(&resp, pretty_print, None);
    println!();
}

/// Handle the project subcommand. Provides facilities for creating a new project,
/// linking a current repository to an existing project, listing projects and
/// setting project thresholds for risk domains.
pub async fn handle_projects(api: &mut PhylumApi, matches: &clap::ArgMatches) -> u8 {
    let pretty_print = !matches.is_present("json");

    if let Some(matches) = matches.subcommand_matches("create") {
        let project_name = matches.value_of("name").unwrap();

        log::info!("Initializing new project: `{}`", project_name);
        // TODO: We shouldn't exit directly from functions or return error codes
        // but use Result and Error, and leave final error handling to main()
        // function.
        let project_id = api
            .create_project(project_name)
            .await
            .unwrap_or_else(|err| exit_error(err, Some("Error initializing project")));

        let proj_conf = ProjectConfig {
            id: project_id.to_owned(),
            name: project_name.to_owned(),
            created_at: Local::now(),
        };

        save_config(PROJ_CONF_FILE, &proj_conf).unwrap_or_else(|err| {
            print_user_failure!("Failed to save user projects file: {}", err);
        });

        print_user_success!("Successfully created new project, {}", project_name);
        return 0;
    } else if matches.subcommand_matches("list").is_some() {
        get_project_list(api, pretty_print).await;
    } else if let Some(matches) = matches.subcommand_matches("link") {
        let project_name = matches.value_of("name").unwrap();
        let resp = api.get_project_details(project_name).await;

        match resp {
            Ok(proj) => {
                let proj_uuid = Uuid::parse_str(proj.id.as_str()).unwrap(); // TODO: Handle this.
                let proj_conf = ProjectConfig {
                    id: proj_uuid,
                    name: proj.name,
                    created_at: Local::now(),
                };
                save_config(PROJ_CONF_FILE, &proj_conf).unwrap_or_else(|err| {
                    log::error!("Failed to save user credentials to config: {}", err)
                });

                print_user_success!(
                    "Linked the current working directory to the project {}.",
                    format!("{}", White.paint(proj_conf.name))
                );
            }
            Err(x) => {
                print_user_failure!("A project with that name does not exist: {}", x);
                return 1;
            }
        }
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
            format!("{}", White.paint("Print a warning"))
        );
        println!(
            "    * {}: If we are in CI/CD break the build and return a non-zero exit code",
            format!("{}", White.paint("Break the build"))
        );
        println!(
            "    * {}: Ignore the failure and continue",
            format!("{}", White.paint("Nothing, fail silently"))
        );
        println!();

        println!("Specify the thresholds and actions for {}. A threshold of zero will disable the threshold.", format!("{}", White.paint(project_name)));
        println!();

        let project_details = match api.get_project_details(project_name).await {
            Ok(x) => x,
            Err(e) => {
                log::error!("Failed to get projet details: {}", e);
                print_user_failure!("Could not get project details");
                return 1;
            }
        };

        let mut user_settings = match api.get_user_settings().await {
            Ok(x) => x,
            Err(e) => {
                log::error!("Failed to get user settings: {}", e);
                print_user_failure!("Could not get user settings");
                return 1;
            }
        };

        for threshold_name in vec![
            "total project",
            "author",
            "engineering",
            "license",
            "malicious code",
            "vulnerability",
        ]
        .iter()
        {
            let (threshold, action) = prompt_threshold(threshold_name).unwrap_or((0, "none"));

            // API expects slight key change for specific fields.
            let name = match *threshold_name {
                "total project" => String::from("total"),
                "malicious code" => String::from("maliciousCode"),
                x => x.to_string(),
            };

            user_settings.set_threshold(
                project_details.id.clone(),
                name,
                threshold,
                action.to_string(),
            );
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
            }
        }
    } else {
        get_project_list(api, pretty_print).await;
    }

    0
}
