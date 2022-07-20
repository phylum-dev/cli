use std::io;
use std::path::Path;
use std::str::FromStr;

use ansi_term::Color::Blue;
use anyhow::{anyhow, Context, Result};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::{Action, JobStatusResponse};
use phylum_types::types::package::{PackageDescriptor, PackageType};
use reqwest::StatusCode;

use crate::api::{PhylumApi, PhylumApiError};
use crate::commands::parse::get_packages_from_lockfile;
use crate::commands::{CommandResult, CommandValue, ExitCode};
use crate::config::{get_current_project, ProjectConfig};
use crate::filter::{Filter, FilterIssues};
use crate::format::Format;
use crate::{print_user_success, print_user_warning};

fn handle_status<T>(resp: Result<JobStatusResponse<T>, PhylumApiError>, pretty: bool) -> Action
where
    JobStatusResponse<T>: Format,
{
    let resp = match resp {
        Ok(resp) => resp,
        Err(err) => {
            if err.status() == Some(StatusCode::NOT_FOUND) {
                print_user_warning!(
                    "No results found. Submit a lockfile for processing:\n\n\t{}\n",
                    Blue.paint("phylum analyze <lock_file>")
                );
            }
            return Action::None;
        },
    };

    resp.write_stdout(pretty);

    if !resp.pass {
        resp.action
    } else {
        Action::None
    }
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
        let mut resp = api.get_job_status_ext(job_id).await;

        if let (Ok(resp), Some(filter)) = (&mut resp, filter) {
            resp.filter(&filter);
        }

        handle_status(resp, pretty)
    } else {
        let resp = api.get_job_status(job_id).await;
        handle_status(resp, pretty)
    }
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
    let display_filter = matches.value_of("filter").and_then(|v| Filter::from_str(v).ok());

    if matches.is_present("JOB_ID") {
        let job_id = JobId::from_str(matches.value_of("JOB_ID").expect("No job id found"))?;
        action = get_job_status(api, &job_id, verbose, pretty_print, display_filter).await;
    } else if let Some(project) = matches.value_of("project") {
        let resp = api.get_project_details(project).await?.jobs;
        resp.write_stdout(pretty_print);
    } else {
        let resp = match api.get_status().await {
            Ok(resp) => resp,
            Err(err) if err.status() == Some(StatusCode::NOT_FOUND) => {
                print_user_warning!(
                    "No results found. Submit a lockfile for processing:\n\n\t{}\n",
                    Blue.paint("phylum analyze <lock_file>")
                );
                return Ok(ExitCode::NoHistoryFound.into());
            },
            Err(err) => return Err(err.into()),
        };

        resp.write_stdout(pretty_print);
    }

    Ok(CommandValue::Action(action))
}

/// Handles submission of packages to the system for analysis and
/// displays summary information about the submitted package(s)
pub async fn handle_submission(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let mut packages = vec![];
    let mut request_type = api.config().request_type; // default request type
    let mut synch = false; // get status after submission
    let mut verbose = false;
    let mut pretty_print = false;
    let mut display_filter = None;
    let mut action = Action::None;
    let is_user; // is a user (non-batch) request
    let project;
    let group;
    let label;

    if let Some(matches) = matches.subcommand_matches("analyze") {
        (project, group) = cli_project(api, matches).await?;

        // Should never get here if `LOCKFILE` was not specified
        let lockfile = matches.value_of("LOCKFILE").ok_or_else(|| anyhow!("Lockfile not found"))?;
        let res = get_packages_from_lockfile(Path::new(lockfile))
            .context("Unable to locate any valid package in package lockfile")?;

        packages = res.0;
        request_type = res.1;

        label = matches.value_of("label");
        verbose = matches.is_present("verbose");
        pretty_print = !matches.is_present("json");
        display_filter = matches.value_of("filter").and_then(|v| Filter::from_str(v).ok());
        is_user = !matches.is_present("force");
        synch = true;
    } else if let Some(matches) = matches.subcommand_matches("batch") {
        (project, group) = cli_project(api, matches).await?;

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
                },
                Err(err) => {
                    return Err(anyhow!(err));
                },
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
            label.map(String::from),
            group.map(String::from),
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

/// Get the current project.
///
/// Assumes that the clap `matches` has a `project` and `group` arguments
/// option.
async fn cli_project(
    api: &mut PhylumApi,
    matches: &clap::ArgMatches,
) -> Result<(ProjectId, Option<String>)> {
    // Prefer `--project` and `--group` if they were specified.
    if let Some(project_name) = matches.value_of("project") {
        let group = matches.value_of("group");
        let project = api.get_project_id(project_name, group).await?;
        return Ok((project, group.map(String::from)));
    }

    // Retrieve the project from the `.phylum_project` file.
    get_current_project().map(|p: ProjectConfig| (p.id, p.group_name)).ok_or_else(|| {
        anyhow!(
            "Failed to find a valid project configuration. Specify an existing project using the \
             `--project` flag, or create a new one with `phylum project create <name>`"
        )
    })
}
