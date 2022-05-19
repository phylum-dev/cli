use std::path::Path;

use crate::api::PhylumApi;
use crate::commands::parse::get_packages_from_lockfile;
use crate::config::get_current_project;

use anyhow::{anyhow, Context, Result};

pub(super) async fn phylum_analyze(api: &mut PhylumApi, lockfile: &str, project: Option<&str>, group: Option<&str>) -> Result<()> {
    let (packages, request_type) = get_packages_from_lockfile(Path::new(lockfile))
        .context("Unable to locate any valid package in package lockfile")?;

    let (project, group) = match (project, group) {
        (Some(project), group) => (api.get_project_id(project, group).await?, None),
        (None, _) => if let Some(p) = get_current_project() {
            (p.id, p.group_name)
        } else {
            return Err(anyhow!("Failed to find a valid project configuration"))
        }
    };

    let job_id = api
        .submit_request(
            &request_type,
            &packages,
            false,
            project,
            None,
            group.map(String::from),
        )
        .await?;

    Ok(())
}
// pub(super) fn phylum_auth_status() -> Result<UserInfo>;
// pub(super) fn phylum_auth_token() -> Result<Token>;
// pub(super) fn phylum_history(job_id: &str) -> Result<()>;
// pub(super) fn phylum_package(name: &str, version: &str, t: Option<Type>) -> Result<PackageStatusExtended>;
// pub(super) fn phylum_parse(lockfile: T, t: Option<LockfileTypeEnum>) -> Vec<PackageDescriptor>;
