use std::path::Path;
use std::str::FromStr;

use crate::commands::parse::{get_packages_from_lockfile, LOCKFILE_PARSERS};
use crate::config::{get_current_project, Config};
use crate::{api::PhylumApi, auth::UserInfo};

use anyhow::{anyhow, Context, Error, Result};
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;

#[allow(unused)]
pub(super) async fn phylum_analyze(
    api: &mut PhylumApi,
    lockfile: &str,
    project: Option<&str>,
    group: Option<&str>,
) -> Result<ProjectId> {
    let (packages, request_type) = get_packages_from_lockfile(Path::new(lockfile))
        .context("Unable to locate any valid package in package lockfile")?;

    let (project, group) = match (project, group) {
        (Some(project), group) => (api.get_project_id(project, group).await?, None),
        (None, _) => {
            if let Some(p) = get_current_project() {
                (p.id, p.group_name)
            } else {
                return Err(anyhow!("Failed to find a valid project configuration"));
            }
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

    Ok(job_id)
}

#[allow(unused)]
pub(super) async fn phylum_auth_status(api: &mut PhylumApi, config: &Config) -> Result<UserInfo> {
    api.user_info(&config.auth_info).await.map_err(Error::from)
}

#[allow(unused)]
pub(super) async fn phylum_auth_token_bearer(config: &Config) -> Result<AccessToken> {
    let refresh_token = phylum_auth_token(config)?;
    let access_token = crate::auth::handle_refresh_tokens(&config.auth_info, &refresh_token)
        .await?
        .access_token;
    Ok(access_token)
}

#[allow(unused)]
pub(super) fn phylum_auth_token(config: &Config) -> Result<RefreshToken> {
    config
        .auth_info
        .offline_access
        .clone()
        .ok_or_else(|| anyhow!("User is not currently authenticated"))
}

#[allow(unused)]
pub(super) async fn phylum_history_job(
    api: &mut PhylumApi,
    job_id: Option<&str>,
) -> Result<JobStatusResponse<PackageStatusExtended>> {
    let job_id = job_id
        .map(|job_id| JobId::from_str(job_id).ok())
        .unwrap_or_else(|| get_current_project().map(|p| p.id))
        .ok_or_else(|| anyhow!("Failed to find a valid project configuration"))?;
    api.get_job_status_ext(&job_id).await.map_err(Error::from)
}

#[allow(unused)]
pub(super) async fn phylum_history_project(
    api: &mut PhylumApi,
    project_name: Option<&str>,
) -> Result<ProjectDetailsResponse> {
    let project_name = project_name
        .map(String::from)
        .map(Result::Ok)
        .unwrap_or_else(|| {
            get_current_project()
                .map(|p| p.name)
                .ok_or_else(|| anyhow!("Failed to find a valid project configuration"))
        })?;
    api.get_project_details(&project_name)
        .await
        .map_err(Error::from)
}

#[allow(unused)]
pub(super) async fn phylum_package(
    api: &mut PhylumApi,
    name: &str,
    version: &str,
    package_type: &str,
) -> Result<Package> {
    let package_type = PackageType::from_str(package_type)
        .map_err(|e| anyhow!("Unrecognized package type `{package_type}`: {e:?}"))?;
    api.get_package_details(&PackageDescriptor {
        name: name.to_string(),
        version: version.to_string(),
        package_type,
    })
    .await
    .map_err(Error::from)
}

#[allow(unused)]
pub(super) fn phylum_parse(lockfile: &str, lockfile_type: &str) -> Result<Vec<PackageDescriptor>> {
    let parser = LOCKFILE_PARSERS
        .iter()
        .find_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
        .ok_or_else(|| anyhow!("Unrecognized lockfile type: `{lockfile_type}`"))?;

    let (pkgs, _) = parser(Path::new(lockfile))?;

    Ok(pkgs)
}
