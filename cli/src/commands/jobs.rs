use std::str::FromStr;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use console::style;
use log::debug;
use phylum_project::DepfileConfig;
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::package::{PackageDescriptor, PackageType};
use reqwest::StatusCode;
#[cfg(feature = "vulnreach")]
use vulnreach_types::{Job, JobPackage};

use crate::api::PhylumApi;
#[cfg(feature = "vulnreach")]
use crate::auth::jwt::RealmRole;
use crate::commands::parse::{self, ParseError};
use crate::commands::{CommandResult, ExitCode};
use crate::format::Format;
#[cfg(feature = "vulnreach")]
use crate::vulnreach;
use crate::{config, print_user_failure, print_user_success, print_user_warning};

/// Output analysis job results.
pub async fn print_job_status(
    api: &PhylumApi,
    job_id: &JobId,
    ignored_packages: impl Into<Vec<PackageDescriptor>>,
    pretty: bool,
) -> CommandResult {
    let response = api.get_job_status_raw(job_id, ignored_packages).await;

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
pub async fn handle_history(api: &PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
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
                    "No results found. Submit a dependency file for processing:\n\n\t{}\n",
                    style("phylum analyze [DEPENDENCY_FILE]").blue()
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
pub async fn handle_submission(api: &PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let mut ignored_packages: Vec<PackageDescriptor> = vec![];
    let mut packages = vec![];
    let mut synch = false; // get status after submission
    let mut pretty_print = false;
    let jobs_project;
    let label;

    if let Some(matches) = matches.subcommand_matches("analyze") {
        let sandbox_generation = !matches.get_flag("skip-sandbox");
        let generate_lockfiles = !matches.get_flag("no-generation");
        label = matches.get_one::<String>("label");
        pretty_print = !matches.get_flag("json");
        synch = true;

        jobs_project = JobsProject::new(api, matches).await?;

        // Get .phylum_project path
        let current_project = phylum_project::get_current_project();
        let project_root = current_project.as_ref().map(|p| p.root());

        for depfile in jobs_project.depfiles {
            let parse_result = parse::parse_depfile(
                &depfile.path,
                project_root,
                Some(&depfile.depfile_type),
                sandbox_generation,
                generate_lockfiles,
            );

            // Map dedicated exit code for failure due to disabled generation.
            let parsed_depfile = match parse_result {
                Ok(parsed_depfile) => parsed_depfile,
                Err(err @ ParseError::ManifestWithoutGeneration(_)) => {
                    print_user_failure!("Could not parse manifest: {}", err);
                    return Ok(ExitCode::ManifestWithoutGeneration);
                },
                Err(ParseError::Other(err)) => {
                    return Err(err).with_context(|| {
                        format!(
                            "Unable to locate any valid package in dependency file {:?}",
                            depfile.path
                        )
                    });
                },
            };

            if pretty_print {
                print_user_success!(
                    "Successfully parsed dependency file {:?} as type {:?}",
                    parsed_depfile.path,
                    parsed_depfile.format.name()
                );
            }

            packages.append(&mut parsed_depfile.api_packages());
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
                        debug!("Invalid package input: `{}`", line);
                        continue;
                    }
                    let pkg_version = pkg_info.pop().unwrap();
                    let pkg_name = pkg_info.join(":");

                    packages.push(
                        PackageDescriptor {
                            name: pkg_name.to_owned(),
                            version: pkg_version.to_owned(),
                            package_type: request_type.to_owned(),
                        }
                        .into(),
                    );
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
        print_user_warning!("No packages found in dependency file");
        return Ok(ExitCode::Ok);
    }

    debug!("Submitting request...");
    let job_id = api
        .submit_request(
            &packages,
            jobs_project.project_id,
            label.map(String::from),
            jobs_project.group,
        )
        .await?;
    debug!("Response => {:?}", job_id);

    if pretty_print {
        print_user_success!("Job ID: {}", job_id);
    }

    if synch {
        if pretty_print {
            #[cfg(feature = "vulnreach")]
            let packages: Vec<_> = packages.into_iter().map(|pkg| pkg.package_descriptor).collect();
            #[cfg(feature = "vulnreach")]
            if let Err(err) = vulnreach(api, matches, packages, job_id.to_string()).await {
                print_user_failure!("Reachability analysis failed: {err:?}");
            }
        }

        debug!("Requesting status...");
        print_job_status(api, &job_id, ignored_packages, pretty_print).await
    } else {
        Ok(ExitCode::Ok)
    }
}

/// Perform vulnerability reachability analysis.
#[cfg(feature = "vulnreach")]
async fn vulnreach(
    api: &PhylumApi,
    matches: &clap::ArgMatches,
    packages: Vec<PackageDescriptor>,
    job_id: String,
) -> Result<()> {
    // Skip requests early if we know user doesn't have the required role.
    let roles = api.roles();
    if !roles.contains(&RealmRole::Vulnreach) {
        debug!("Skipping reachability analysis: User roles missing `vulnreach`: ({roles:?})");
        return Ok(());
    }

    // Find all direct dependencies.
    let local_imports = vulnreach::imports();

    // Output reachability results.
    let imports = job_from_packages(packages, local_imports, job_id);
    let vulnerabilities = api.vulnerabilities(imports).await?;

    // Skip output if there are no vulnerabilities.
    if vulnerabilities.is_empty() {
        return Ok(());
    }

    println!("{}", style("\n# Identified vulnerabilities").bold());

    // Output reachability for each individual vulnerability.
    for vulnerability in &vulnerabilities {
        println!();

        if matches.get_count("verbose") > 0 {
            vulnerability.pretty_verbose(&mut io::stdout());
        } else {
            vulnerability.pretty(&mut io::stdout());
        }
    }

    println!();

    Ok(())
}

/// Project information for analyze/batch.
struct JobsProject {
    project_id: ProjectId,
    group: Option<String>,
    depfiles: Vec<DepfileConfig>,
}

impl JobsProject {
    /// Get the current project.
    ///
    /// Assumes that the clap `matches` has a `project` and `group` arguments
    /// option.
    async fn new(api: &PhylumApi, matches: &clap::ArgMatches) -> Result<JobsProject> {
        let current_project = phylum_project::get_current_project();
        let depfiles = config::depfiles(matches, current_project.as_ref())?;

        match matches.get_one::<String>("project") {
            // Prefer `--project` and `--group` if they were specified.
            Some(project_name) => {
                let group = matches.get_one::<String>("group").cloned();
                let project = api.get_project_id(project_name, group.as_deref()).await?;
                Ok(Self { project_id: project, group, depfiles })
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
                    depfiles,
                })
            },
        }
    }
}

/// Convert Vec<PackageDescriptor> to Imports.
#[cfg(feature = "vulnreach")]
fn job_from_packages(
    dependencies: Vec<PackageDescriptor>,
    imports: Vec<String>,
    analysis_job_id: String,
) -> Job {
    let dependencies = dependencies
        .into_iter()
        .map(|package| JobPackage {
            name: package.name,
            version: package.version,
            ecosystem: package.package_type.to_string(),
        })
        .collect();

    Job { analysis_job_id, dependencies, imported_packages: imports.into_iter().collect() }
}
