use std::io;
use std::path::Path;
use std::str::FromStr;

use ansi_term::Color::Blue;
use anyhow::{anyhow, Context, Result};
use phylum_types::types::common::Status;
use reqwest::StatusCode;
use serde::Serialize;
use uuid::Uuid;

use phylum_types::types::common::JobId;
use phylum_types::types::job::*;
use phylum_types::types::package::*;

use crate::api::{PhylumApi, PhylumApiError};
use crate::commands::lock_files::get_packages_from_lockfile;
use crate::commands::{CommandResult, CommandValue};
use crate::config::{get_current_project, Config, ProjectConfig};
use crate::filter::Filter;
use crate::print::print_response;
use crate::print_user_success;
use crate::print_user_warning;
use crate::summarize::Summarize;

use super::project::get_project_list;

fn handle_status<T>(
    resp: Result<JobStatusResponse<T>, PhylumApiError>,
    pretty: bool,
    filter: Option<Filter>,
) -> Action
where
    T: std::fmt::Debug + Serialize + Summarize,
    JobStatusResponse<T>: Summarize,
{
    let mut action = Action::None;

    if let Err(Some(StatusCode::NOT_FOUND)) = resp.as_ref().map_err(|e| e.status()) {
        print_user_warning!(
            "No results found. Submit a lockfile for processing:\n\n\t{}\n",
            Blue.paint("phylum analyze <lock_file>")
        );
    } else {
        if let Ok(ref resp) = resp {
            if resp.status == Status::Complete && !resp.pass {
                action = resp.action.to_owned();
            }
        }
        print_response(&resp, pretty, filter);
    }

    action
}

/// Display user-friendly overview of a job
pub async fn get_job_status(
    api: &mut PhylumApi,
    job_id: &JobId,
    verbose: bool,
    pretty: bool,
    filter: Option<Filter>,
) -> Action {
    if verbose {
        let resp = api.get_job_status_ext(job_id).await;
        handle_status(resp, pretty, filter)
    } else {
        let resp = api.get_job_status(job_id).await;
        handle_status(resp, pretty, filter)
    }
}

/// Resolve a potential job_id, which could be a UUID string or the value
/// 'current' which means the UUID of the current running job.
fn resolve_job_id(job_id: &str) -> Result<Uuid> {
    let maybe_job_id = if job_id == "current" {
        get_current_project().map(|p: ProjectConfig| p.id)
    } else {
        JobId::from_str(job_id).ok()
    };

    maybe_job_id.ok_or_else(|| anyhow!("Unable to resolve, or invalid job id: {}", job_id))
}

/// Handle the history subcommand.
///
/// This allows us to list last N job runs, list the projects, list runs
/// associated with projects, and get the detailed run results for a specific
/// job run.
pub async fn handle_history(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let pretty_print = !matches.is_present("json");
    let verbose = matches.is_present("verbose");
    let mut action = Action::None;
    let display_filter = matches
        .value_of("filter")
        .and_then(|v| Filter::from_str(v).ok());

    if let Some(matches) = matches.subcommand_matches("project") {
        let project_name = matches.value_of("project_name");
        let project_job_id = matches.value_of("job_id");

        if let Some(project_name) = project_name {
            if project_job_id.is_none() {
                print_user_warning!(
                    "`phylum history project <PROJECT>` is deprecated, \
                    use `phylum history --project <PROJECT>` instead"
                );

                let resp = api.get_project_details(project_name).await;
                print_response(&resp, pretty_print, None);
            } else {
                print_user_warning!(
                    "`phylum history project <PROJECT> <JOB_ID>` is deprecated, \
                    use `phylum history <JOB_ID>` instead"
                );

                // TODO The original code had unwrap in it above. This needs to
                // be refactored in general for better flow
                let job_id = resolve_job_id(project_job_id.expect("No job id found"))?;
                action = get_job_status(api, &job_id, verbose, pretty_print, display_filter).await
            }
        } else {
            print_user_warning!(
                "`phylum history project` is deprecated, use `phylum project` instead"
            );

            get_project_list(api, pretty_print).await;
        }
    } else if matches.is_present("JOB_ID") {
        let job_id = resolve_job_id(matches.value_of("JOB_ID").expect("No job id found"))?;
        action = get_job_status(api, &job_id, verbose, pretty_print, display_filter).await;
    } else if let Some(project) = matches.value_of("project") {
        let resp = api.get_project_details(project).await.map(|r| r.jobs);
        print_response(&resp, pretty_print, None);
    } else {
        let resp = api.get_status().await;

        if let Err(Some(StatusCode::NOT_FOUND)) = resp.as_ref().map_err(|e| e.status()) {
            print_user_warning!(
                "No results found. Submit a lockfile for processing:\n\n\t{}\n",
                Blue.paint("phylum analyze <lock_file>")
            );
        } else {
            if pretty_print {
                println!("Projects and most recent runs\n",);
            }

            print_response(&resp, pretty_print, None);
        }
    }

    Ok(CommandValue::Action(action))
}

/// Handles submission of packages to the system for analysis and
/// displays summary information about the submitted package(s)
pub async fn handle_submission(
    api: &mut PhylumApi,
    config: Config,
    matches: &clap::ArgMatches,
) -> CommandResult {
    let mut packages = vec![];
    let mut request_type = config.request_type; // default request type
    let mut synch = false; // get status after submission
    let mut verbose = false;
    let mut pretty_print = false;
    let mut display_filter = None;
    let mut action = Action::None;
    let is_user; // is a user (non-batch) request
    let project;
    let label;

    if let Some(matches) = matches.subcommand_matches("analyze") {
        project = project_uuid(api, matches).await?;

        // Should never get here if `LOCKFILE` was not specified
        let lockfile = matches
            .value_of("LOCKFILE")
            .ok_or_else(|| anyhow!("Lockfile not found"))?;
        let res = get_packages_from_lockfile(Path::new(lockfile))
            .context("Unable to locate any valid package in package lockfile")?;

        packages = res.0;
        request_type = res.1;

        label = matches.value_of("label");
        verbose = matches.is_present("verbose");
        pretty_print = !matches.is_present("json");
        display_filter = matches
            .value_of("filter")
            .and_then(|v| Filter::from_str(v).ok());
        is_user = !matches.is_present("force");
        synch = true;
    } else if let Some(matches) = matches.subcommand_matches("batch") {
        project = project_uuid(api, matches).await?;

        let mut eof = false;
        let mut line = String::new();
        let mut reader: Box<dyn io::BufRead> = if let Some(file) = matches.value_of("file") {
            // read entries from the file
            Box::new(io::BufReader::new(std::fs::File::open(file).unwrap()))
        } else {
            // read from stdin
            log::info!("Waiting on stdin...");
            Box::new(io::BufReader::new(io::stdin()))
        };

        // If a package type was provided on the command line, prefer that
        //  to the global setting
        if matches.is_present("type") {
            request_type =
                PackageType::from_str(matches.value_of("type").unwrap()).unwrap_or(request_type);
        }
        label = matches.value_of("label");
        is_user = !matches.is_present("force");

        while !eof {
            match reader.read_line(&mut line) {
                Ok(0) => eof = true,
                Ok(_) => {
                    line.pop();
                    let mut pkg_info = line.split(':').collect::<Vec<&str>>();
                    if pkg_info.len() < 2 {
                        log::debug!("Invalid package input: `{}`", line);
                        continue;
                    }
                    let pkg_version = pkg_info.pop().unwrap();
                    let pkg_name = pkg_info.join(":");

                    packages.push(PackageDescriptor {
                        name: pkg_name.to_owned(),
                        version: pkg_version.to_owned(),
                        package_type: request_type.to_owned(),
                    });
                    line.clear();
                }
                Err(err) => {
                    return Err(anyhow!(err));
                }
            }
        }
    } else {
        unreachable!();
    }

    log::debug!("Submitting request...");
    let job_id = api
        .submit_request(
            &request_type,
            &packages,
            is_user,
            project,
            label.map(|s| s.to_string()),
        )
        .await?;

    log::debug!("Response => {:?}", job_id);
    print_user_success!("Job ID: {}", job_id);

    if synch {
        log::debug!("Requesting status...");
        action = get_job_status(api, &job_id, verbose, pretty_print, display_filter).await;
    }
    Ok(CommandValue::Action(action))
}

/// Get the current project's UUID.
///
/// Assumes that the clap `matches` has a `project` arguments option.
async fn project_uuid(api: &mut PhylumApi, matches: &clap::ArgMatches) -> Result<Uuid> {
    // Prefer `--project` if it was specified.
    if let Some(project_name) = matches.value_of("project") {
        let response = api.get_project_details(project_name).await;
        let project_id = response.context("Project details request failure")?.id;
        return Uuid::parse_str(&project_id).context("Invalid project UUID");
    }

    // Retrieve the project from the `.phylum_project` file.
    get_current_project()
        .map(|p: ProjectConfig| p.id)
        .ok_or_else(|| {
            anyhow!(
                "Failed to find a valid project configuration. Specify an existing project using \
                the `--project` flag, or create a new one with `phylum project create <name>`"
            )
        })
}
