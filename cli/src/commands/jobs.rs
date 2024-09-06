use std::fs;
#[cfg(feature = "vulnreach")]
use std::io;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use console::style;
use log::debug;
use phylum_lockfile::ParseError;
use phylum_project::DepfileConfig;
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::package::PackageDescriptor;
use reqwest::StatusCode;
#[cfg(feature = "vulnreach")]
use vulnreach_types::{Job, JobPackage};

use crate::api::PhylumApi;
#[cfg(feature = "vulnreach")]
use crate::auth::jwt::RealmRole;
use crate::commands::{parse, CommandResult, ExitCode};
use crate::format::Format;
use crate::types::AnalysisPackageDescriptor;
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

/// Handle `phylum analyze` subcommand.
pub async fn handle_analyze(api: &PhylumApi, matches: &clap::ArgMatches) -> CommandResult {
    let sandbox_generation = !matches.get_flag("skip-sandbox");
    let generate_lockfiles = !matches.get_flag("no-generation");
    let label = matches.get_one::<String>("label");
    let pretty_print = !matches.get_flag("json");

    let jobs_project = JobsProject::new(api, matches).await?;

    // Get .phylum_project path.
    let current_project = phylum_project::get_current_project();
    let project_root = current_project.as_ref().map(|p| p.root());

    let mut packages = Vec::new();
    for depfile in jobs_project.depfiles {
        let parse_result = parse::parse_depfile(
            &depfile.path,
            project_root,
            Some(&depfile.depfile_type),
            sandbox_generation,
            generate_lockfiles,
        );

        // Map dedicated exit codes for failures due to disabled generation or
        // unknown dependency file format.
        let parsed_depfile = match parse_result {
            Ok(parsed_depfile) => parsed_depfile,
            Err(err @ ParseError::ManifestWithoutGeneration(_)) => {
                print_user_failure!("Could not parse manifest: {}", err);
                return Ok(ExitCode::ManifestWithoutGeneration);
            },
            Err(err @ ParseError::UnknownManifestFormat(_)) => {
                print_user_failure!("Could not parse manifest: {}", err);
                return Ok(ExitCode::UnknownManifestFormat);
            },
            Err(ParseError::Other(err)) => {
                return Err(err).with_context(|| {
                    format!(
                        "Could not parse dependency file {:?} as {:?} type",
                        depfile.path.display(),
                        depfile.depfile_type
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

        let mut analysis_packages =
            AnalysisPackageDescriptor::descriptors_from_lockfile(parsed_depfile);
        packages.append(&mut analysis_packages);
    }

    let ignored_packages: Vec<PackageDescriptor> = match matches.get_one::<String>("base") {
        Some(base) => {
            let base_text = fs::read_to_string(base)?;
            serde_json::from_str(&base_text)?
        },
        None => Vec::new(),
    };

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

        #[cfg(feature = "vulnreach")]
        let packages: Vec<_> = packages
            .into_iter()
            .filter_map(|pkg| match pkg {
                AnalysisPackageDescriptor::PackageDescriptor(package) => {
                    Some(package.package_descriptor)
                },
                AnalysisPackageDescriptor::Purl(_) => None,
            })
            .collect();
        #[cfg(feature = "vulnreach")]
        if let Err(err) = vulnreach(api, matches, packages, job_id.to_string()).await {
            print_user_failure!("Reachability analysis failed: {err:?}");
        }
    }

    debug!("Requesting status...");
    print_job_status(api, &job_id, ignored_packages, pretty_print).await
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

/// Project information for analyze.
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
