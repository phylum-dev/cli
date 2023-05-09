use std::str::FromStr;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use console::style;
use phylum_project::LockfileConfig;
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::package::{PackageDescriptor, PackageType};
use reqwest::StatusCode;

use crate::api::PhylumApi;
use crate::commands::{parse, CommandResult, ExitCode};
use crate::format::Format;
use crate::{config, print_user_success, print_user_warning};

/// Output analysis job results.
pub async fn print_job_status(
    api: &mut PhylumApi,
    job_id: &JobId,
    ignored_packages: impl Into<Vec<PackageDescriptor>>,
    pretty: bool,
) -> CommandResult {
    let response = api.get_job_status(job_id, ignored_packages).await;

    // Provide nicer messages for specific errors.
    let status = match response {
        Ok(status) => status,
        Err(err) if err.status() == Some(StatusCode::NOT_FOUND) => {
            print_user_warning!("No results found for JobId {job_id}.");
            return Ok(ExitCode::NotFound);
        },
        Err(err) => return Err(err.into()),
    };

    status.write_stdout(pretty);

    if status.is_failure {
        Ok(ExitCode::FailedPolicy)
    } else {
        Ok(ExitCode::Ok)
    }
}

/// Handle the history subcommand.
///
/// This allows us to list last N job runs, list the projects, list runs
/// associated with projects, and get the detailed run results for a specific
/// job run.
pub async fn handle_history(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let pretty_print = !matches.get_flag("json");

    if let Some(job_id) = matches.get_one::<String>("JOB_ID") {
        let job_id =
            JobId::from_str(job_id).with_context(|| format!("{job_id:?} is not a valid Job ID"))?;
        return print_job_status(api, &job_id, [], pretty_print).await;
    } else if let Some(project) = matches.get_one::<String>("project") {
        let group = matches.get_one::<String>("group").map(String::as_str);
        let history = api.get_project_history(project, group).await?;
        history.write_stdout(pretty_print);
    } else {
        let resp = match api.get_status().await {
            Ok(resp) => resp,
            Err(err) if err.status() == Some(StatusCode::NOT_FOUND) => {
                print_user_warning!(
                    "No results found. Submit a lockfile for processing:\n\n\t{}\n",
                    style("phylum analyze <lock_file>").blue()
                );
                return Ok(ExitCode::NoHistoryFound);
            },
            Err(err) => return Err(err.into()),
        };

        resp.write_stdout(pretty_print);
    }

    Ok(ExitCode::Ok)
}

/// Handles submission of packages to the system for analysis and
/// displays summary information about the submitted package(s)
pub async fn handle_submission(api: &mut PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let mut ignored_packages: Vec<PackageDescriptor> = vec![];
    let mut packages = vec![];
    let mut synch = false; // get status after submission
    let mut pretty_print = false;
    let jobs_project;
    let label;

    if let Some(matches) = matches.subcommand_matches("analyze") {
        label = matches.get_one::<String>("label");
        pretty_print = !matches.get_flag("json");
        synch = true;

        jobs_project = JobsProject::new(api, matches).await?;

        for lockfile in jobs_project.lockfiles {
            let res = parse::parse_lockfile(&lockfile.path, Some(&lockfile.lockfile_type))
                .with_context(|| {
                    format!("Unable to locate any valid package in lockfile {:?}", lockfile.path)
                })?;

            if pretty_print {
                print_user_success!(
                    "Successfully parsed lockfile {:?} as type: {}",
                    res.path,
                    res.format.name()
                );
            }

            packages.extend(res.packages.into_iter());
        }

        if let Some(base) = matches.get_one::<String>("base") {
            let base_text = fs::read_to_string(base)?;
            ignored_packages = serde_json::from_str(&base_text)?;
        }
    } else if let Some(matches) = matches.subcommand_matches("batch") {
        jobs_project = JobsProject::new(api, matches).await?;

        let mut eof = false;
        let mut line = String::new();
        let mut reader: Box<dyn io::BufRead> = if let Some(file) = matches.get_one::<String>("file")
        {
            // read entries from the file
            Box::new(io::BufReader::new(std::fs::File::open(file).unwrap()))
        } else {
            // read from stdin
            log::info!("Waiting on stdin...");
            Box::new(io::BufReader::new(io::stdin()))
        };

        let request_type = {
            let package_type = matches.get_one::<String>("type").unwrap();
            PackageType::from_str(package_type)
                .map_err(|_| anyhow!("invalid package type: {}", package_type))?
        };

        label = matches.get_one::<String>("label");

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

    // Avoid request error without dependencies.
    if packages.is_empty() {
        print_user_warning!("No packages found in lockfile");
        return Ok(ExitCode::Ok);
    }

    log::debug!("Submitting request...");
    let job_id = api
        .submit_request(
            &packages,
            jobs_project.project_id,
            label.map(String::from),
            jobs_project.group,
        )
        .await?;
    log::debug!("Response => {:?}", job_id);

    if pretty_print {
        print_user_success!("Job ID: {}", job_id);
    }

    if synch {
        log::debug!("Requesting status...");
        print_job_status(api, &job_id, ignored_packages, pretty_print).await
    } else {
        Ok(ExitCode::Ok)
    }
}

/// Project information for analyze/batch.
struct JobsProject {
    project_id: ProjectId,
    group: Option<String>,
    lockfiles: Vec<LockfileConfig>,
}

impl JobsProject {
    /// Get the current project.
    ///
    /// Assumes that the clap `matches` has a `project` and `group` arguments
    /// option.
    async fn new(api: &mut PhylumApi, matches: &clap::ArgMatches) -> Result<JobsProject> {
        let current_project = phylum_project::get_current_project();
        let lockfiles = config::lockfiles(matches, current_project.as_ref())?;

        match matches.get_one::<String>("project") {
            // Prefer `--project` and `--group` if they were specified.
            Some(project_name) => {
                let group = matches.get_one::<String>("group").cloned();
                let project = api.get_project_id(project_name, group.as_deref()).await?;
                Ok(Self { project_id: project, group, lockfiles })
            },
            // Retrieve the project from the `.phylum_project` file.
            None => {
                let current_project = current_project.ok_or_else(|| {
                    anyhow!(
                        "Failed to find a valid project configuration. Specify an existing \
                         project using the `--project` flag, or create a new one with `phylum \
                         project create <name>`"
                    )
                })?;

                Ok(Self {
                    project_id: current_project.id,
                    group: current_project.group_name,
                    lockfiles,
                })
            },
        }
    }
}
