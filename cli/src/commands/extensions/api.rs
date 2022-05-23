//! Most functions in this module are marked `#[allow(unused)]` for the time being. This
//! intentional, as there are no clients for those functions in the rest of the code, but
//! those functions will have to be used by the Deno integration; at that point, we may
//! remove the annotations.

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
    #[allow(unused)]
    pub(super) fn from_factories(
        api_factory: fn() -> PhylumApi,
        config_factory: fn() -> Config,
    ) -> Self {
        InjectedDependencies {
            api: Lazy::new(api_factory),
            config: Lazy::new(config_factory),
        }
    }
}

#[allow(unused)]
pub(super) async fn phylum_analyze(
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

#[allow(unused)]
pub(super) async fn phylum_auth_status(state: &mut OpState) -> Result<UserInfo> {
    let deps = state.borrow_mut::<InjectedDependencies>();
    deps.api
        .user_info(&deps.config.auth_info)
        .await
        .map_err(Error::from)
}

#[allow(unused)]
pub(super) async fn phylum_auth_token_bearer(
    state: &mut OpState,
    ignore_certs: bool,
) -> Result<AccessToken> {
    let refresh_token = phylum_auth_token(state)?;
    let config = &mut state.borrow_mut::<InjectedDependencies>().config;
    let access_token =
        crate::auth::handle_refresh_tokens(&config.auth_info, &refresh_token, ignore_certs)
            .await?
            .access_token;
    Ok(access_token)
}

#[allow(unused)]
pub(super) fn phylum_auth_token(state: &mut OpState) -> Result<RefreshToken> {
    let config = &mut state.borrow_mut::<InjectedDependencies>().config;
    config
        .auth_info
        .offline_access
        .clone()
        .ok_or_else(|| anyhow!("User is not currently authenticated"))
}

#[allow(unused)]
pub(super) async fn phylum_history_job(
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

#[allow(unused)]
pub(super) async fn phylum_history_project(
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

#[allow(unused)]
pub(super) async fn phylum_package(
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

#[allow(unused)]
pub(super) fn phylum_parse(lockfile: &str, lockfile_type: &str) -> Result<Vec<PackageDescriptor>> {
    let parser = LOCKFILE_PARSERS
        .iter()
        .find_map(|(name, parser)| (*name == lockfile_type).then(|| parser))
        .ok_or_else(|| anyhow!("Unrecognized lockfile type: `{lockfile_type}`"))?;

    let (pkgs, _) = parser(Path::new(lockfile))?;

    Ok(pkgs)
}
