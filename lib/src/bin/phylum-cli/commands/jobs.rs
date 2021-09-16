use std::io;
use std::str::FromStr;

use ansi_term::Color::Blue;
use phylum_cli::config::{get_current_project, Config, ProjectConfig};
use serde::Serialize;

use phylum_cli::api::PhylumApi;
use phylum_cli::summarize::Summarize;
use phylum_cli::types::{Action, JobId, PackageDescriptor, PackageType, RequestStatusResponse};

use crate::commands::lock_files::get_packages_from_lockfile;
use crate::exit::{err_exit, exit};
use crate::print::print_response;
use crate::print_user_success;
use crate::print_user_warning;

use super::projects::get_project_list;

fn handle_status<T>(
    resp: Result<RequestStatusResponse<T>, phylum_cli::Error>,
    pretty: bool,
) -> Action
where
    T: std::fmt::Debug + Serialize + Summarize,
    phylum_cli::types::RequestStatusResponse<T>: Summarize,
{
    let mut action = Action::None;

    if let Err(phylum_cli::Error::HttpError(404, _)) = resp {
        print_user_warning!(
            "No results found. Submit a lockfile for processing:\n\n\t{}\n",
            Blue.paint("phylum analyze <lock_file>")
        );
    } else {
        if let Ok(ref resp) = resp {
            if !resp.pass {
                action = resp.action.to_owned();
            }
        }
        print_response(&resp, pretty);
    }

    action
}

/// Display user-friendly overview of a job
pub fn get_job_status(api: &mut PhylumApi, job_id: &JobId, verbose: bool, pretty: bool) -> Action {
    if verbose {
        let resp = api.get_job_status_ext(job_id);
        handle_status(resp, pretty)
    } else {
        let resp = api.get_job_status(job_id);
        handle_status(resp, pretty)
    }
}

/// Handle the history subcommand.
///
/// This allows us to list last N job runs, list the projects, list runs
/// associated with projects, and get the detailed run results for a specific
/// job run.
pub fn handle_history(api: &mut PhylumApi, config: Config, matches: &clap::ArgMatches) -> Action {
    let pretty_print = !matches.is_present("json");
    let verbose = matches.is_present("verbose");
    let mut ret = Action::None;

    let mut get_job = |job_id: Option<&str>| {
        let job_id_str = job_id.unwrap();

        let job_id = if job_id_str == "current" {
            get_current_project().map(|p: ProjectConfig| p.id)
        } else {
            JobId::from_str(job_id_str).ok()
        }
        .unwrap_or_else(|| exit(Some(&format!("Invalid job id: {}", job_id_str)), -3));

        get_job_status(api, &job_id, verbose, pretty_print)
    };

    if let Some(matches) = matches.subcommand_matches("project") {
        let project_name = matches.value_of("project_name");
        let project_job_id = matches.value_of("job_id");

        if let Some(project_name) = project_name {
            if project_job_id.is_none() {
                let resp = api.get_project_details(project_name);
                print_response(&resp, pretty_print);
            } else {
                ret = get_job(project_job_id);
            }
        } else {
            get_project_list(api, pretty_print);
        }
    } else if matches.is_present("JOB_ID") {
        ret = get_job(matches.value_of("JOB_ID"));
    } else {
        let resp = api.get_status();
        if let Err(phylum_cli::Error::HttpError(404, _)) = resp {
            print_user_warning!(
                "No results found. Submit a lockfile for processing:\n\n\t{}\n",
                Blue.paint("phylum analyze <lock_file>")
            );
        } else {
            println!(
                "Projects and most recent run for {}\n",
                Blue.paint(&config.auth_info.user)
            );
            print_response(&resp, pretty_print);
        }
    }

    ret
}

/// Handles submission of packages to the system for analysis and
/// displays summary information about the submitted package(s)
pub fn handle_submission(
    api: &mut PhylumApi,
    config: Config,
    matches: &clap::ArgMatches,
) -> Action {
    let mut packages = vec![];
    let mut request_type = config.request_type; // default request type
    let mut synch = false; // get status after submission
    let mut verbose = false;
    let mut pretty_print = false;
    let mut label = None;
    let mut is_user = true; // is a user (non-batch) request
    let mut ret = Action::None;

    let project = get_current_project()
        .map(|p: ProjectConfig| p.id)
        .unwrap_or_else(|| {
            exit(
                Some("Failed to find a valid project configuration. Did you run `phylum projects create <project-name>`?"),
                -1
            );
        });

    if let Some(matches) = matches.subcommand_matches("analyze") {
        // Should never get here if `LOCKFILE` was not specified
        let lockfile = matches.value_of("LOCKFILE").unwrap();
        let res = get_packages_from_lockfile(lockfile).unwrap_or_else(|| {
            exit(
                Some("Unable to locate any valid package in package lockfile"),
                -1,
            )
        });

        packages = res.0;
        request_type = res.1;

        label = matches.value_of("label");
        verbose = matches.is_present("verbose");
        pretty_print = !matches.is_present("json");
        is_user = !matches.is_present("force");
        synch = true;
    } else if let Some(matches) = matches.subcommand_matches("batch") {
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
                    let pkg_info = line.split(':').collect::<Vec<&str>>();
                    if pkg_info.len() != 2 {
                        log::debug!("Invalid package input: `{}`", line);
                        continue;
                    }
                    packages.push(PackageDescriptor {
                        name: pkg_info[0].to_owned(),
                        version: pkg_info[1].to_owned(),
                        r#type: request_type.to_owned(),
                    });
                    line.clear();
                }
                Err(err) => {
                    err_exit(err, "Error reading input", -6);
                }
            }
        }
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
        .unwrap_or_else(|err| err_exit(err, "Error submitting package", -2));

    log::debug!("Response => {:?}", job_id);
    print_user_success!("Job ID: {}", job_id);

    if synch {
        log::debug!("Requesting status...");
        ret = get_job_status(api, &job_id, verbose, pretty_print);
    }

    ret
}
