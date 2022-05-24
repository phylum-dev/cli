//! TODO remove the following annotation before merging. The functions in
//! this module should have as unique client the Deno runtime; pending its
//! implementation, having these functions unused would break CI.
#![allow(unused)]

use std::path::Path;
use std::str::FromStr;

use crate::commands::parse::{get_packages_from_lockfile, LOCKFILE_PARSERS};
use crate::config::{get_current_project, Config};
use crate::{api::PhylumApi, auth::UserInfo};

use anyhow::{anyhow, Context, Error, Result};
use deno_core::OpState;
use once_cell::sync::Lazy;
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;

/// Container for lazily evaluated dependencies of Extensions API functions. These values won't be
/// visible from extension code and will have to be set up by the JS runtime builder which will
/// load their configuration as any other command and then provide the pertinent factories.
struct InjectedDependencies {
    api: Lazy<PhylumApi>,
    config: Lazy<Config>,
}

impl InjectedDependencies {
    pub(crate) fn from_factories(
        api_factory: fn() -> PhylumApi,
        config_factory: fn() -> Config,
    ) -> Self {
        InjectedDependencies {
            api: Lazy::new(api_factory),
            config: Lazy::new(config_factory),
        }
    }
}

/// Analyze a lockfile.
/// Equivalent to `phylum analyze`.
pub(crate) async fn analyze(
    state: &mut OpState,
    lockfile: &str,
    project: Option<&str>,
    group: Option<&str>,
) -> Result<ProjectId> {
    let api = &mut state.borrow_mut::<InjectedDependencies>().api;
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

/// Retrieve user info.
/// Equivalent to `phylum auth status`.
pub(crate) async fn get_user_info(state: &mut OpState) -> Result<UserInfo> {
    let deps = state.borrow_mut::<InjectedDependencies>();
    deps.api.user_info().await.map_err(Error::from)
}

/// Retrieve the access token.
/// Equivalent to `phylum auth token --bearer`.
pub(crate) async fn get_access_token(
    state: &mut OpState,
    ignore_certs: bool,
) -> Result<AccessToken> {
    let refresh_token = get_refresh_token(state)?;
    let config = &mut state.borrow_mut::<InjectedDependencies>().config;
    let access_token =
        crate::auth::handle_refresh_tokens(&refresh_token, ignore_certs, &config.connection.uri)
            .await?
            .access_token;
    Ok(access_token)
}

/// Retrieve the refresh token.
/// Equivalent to `phylum auth token`.
pub(crate) fn get_refresh_token(state: &mut OpState) -> Result<RefreshToken> {
    let config = &mut state.borrow_mut::<InjectedDependencies>().config;
    config
        .auth_info
        .offline_access
        .clone()
        .ok_or_else(|| anyhow!("User is not currently authenticated"))
}

/// Retrieve a job's status.
/// Equivalent to `phylum history job`.
pub(crate) async fn get_job_status(
    state: &mut OpState,
    job_id: Option<&str>,
) -> Result<JobStatusResponse<PackageStatusExtended>> {
    let api = &mut state.borrow_mut::<InjectedDependencies>().api;
    let job_id = job_id
        .map(|job_id| JobId::from_str(job_id).ok())
        .unwrap_or_else(|| get_current_project().map(|p| p.id))
        .ok_or_else(|| anyhow!("Failed to find a valid project configuration"))?;
    api.get_job_status_ext(&job_id).await.map_err(Error::from)
}

/// Retrieve a project's details.
/// Equivalent to `phylum history project`.
pub(crate) async fn get_project_details(
    state: &mut OpState,
    project_name: Option<&str>,
) -> Result<ProjectDetailsResponse> {
    let api = &mut state.borrow_mut::<InjectedDependencies>().api;
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

/// Analyze a single package.
/// Equivalent to `phylum package`.
pub(crate) async fn analyze_package(
    state: &mut OpState,
    name: &str,
    version: &str,
    package_type: &str,
) -> Result<Package> {
    let api = &mut state.borrow_mut::<InjectedDependencies>().api;
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

/// Parse a lockfile and return the package descriptors contained therein.
/// Equivalent to `phylum parse`.
pub(crate) fn parse_lockfile(
    lockfile: &str,
    lockfile_type: &str,
) -> Result<Vec<PackageDescriptor>> {
    let parser = LOCKFILE_PARSERS
        .iter()
        .find_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
        .ok_or_else(|| anyhow!("Unrecognized lockfile type: `{lockfile_type}`"))?;

    let (pkgs, _) = parser(Path::new(lockfile))?;

    Ok(pkgs)
}
