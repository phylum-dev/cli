//! Subcommand `phylum status`.

use std::path::PathBuf;

use chrono::{DateTime, Local};
use clap::ArgMatches;
use phylum_project::LockfileConfig;
use phylum_types::types::common::ProjectId;
use serde::Serialize;

use crate::commands::{CommandResult, ExitCode};
use crate::config;
use crate::format::Format;

pub async fn handle_status(matches: &ArgMatches) -> CommandResult {
    let pretty_print = !matches.get_flag("json");
    let status = PhylumStatus::new(matches);
    status.write_stdout(pretty_print);
    Ok(ExitCode::Ok)
}

#[derive(Serialize, Default)]
pub struct PhylumStatus {
    pub lockfiles: Vec<LockfileConfig>,
    pub project: Option<String>,
    pub group: Option<String>,
    pub root: Option<PathBuf>,

    // JSON-only fields:
    created_at: Option<DateTime<Local>>,
    id: Option<ProjectId>,
}

impl PhylumStatus {
    fn new(matches: &ArgMatches) -> Self {
        let mut status = PhylumStatus::default();
        let project = phylum_project::get_current_project();

        // Add lockfiles.
        status.lockfiles = config::lockfiles(matches, project.as_ref()).unwrap_or_default();

        // Populate project details.
        if let Some(project) = project {
            status.created_at = Some(project.created_at);
            status.root = Some(project.root().clone());
            status.project = Some(project.name);
            status.group = project.group_name;
            status.id = Some(project.id);
        }

        status
    }
}
