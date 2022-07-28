//! Extension API functions.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{anyhow, Context, Error, Result};
use deno_runtime::deno_core::{op, OpDecl, OpState};
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::JobId;
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::auth::UserInfo;
use crate::commands::extensions::state::ExtensionState;
use crate::commands::parse::{self, LOCKFILE_PARSERS};
use crate::config::get_current_project;

/// Package descriptor for any ecosystem.
#[derive(Serialize, Deserialize, Debug)]
struct PackageSpecifier {
    name: String,
    version: String,
}

impl From<PackageDescriptor> for PackageSpecifier {
    fn from(descriptor: PackageDescriptor) -> Self {
        Self { name: descriptor.name, version: descriptor.version }
    }
}

/// Parsed lockfile content.
#[derive(Serialize, Deserialize, Debug)]
struct PackageLock {
    packages: Vec<PackageSpecifier>,
    package_type: PackageType,
}

/// Analyze a lockfile.
///
/// Equivalent to `phylum analyze`.
#[op]
async fn analyze(
    op_state: Rc<RefCell<OpState>>,
    package_type: PackageType,
    packages: Vec<PackageSpecifier>,
    project: Option<String>,
    group: Option<String>,
) -> Result<JobId> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let (project, group) = match (project, group) {
        (Some(project), group) => (api.get_project_id(&project, group.as_deref()).await?, None),
        (None, _) => {
            if let Some(p) = get_current_project() {
                (p.id, p.group_name)
            } else {
                return Err(anyhow!("Failed to find a valid project configuration"));
            }
        },
    };

    let packages = packages
        .into_iter()
        .map(|package| PackageDescriptor {
            package_type,
            version: package.version,
            name: package.name,
        })
        .collect::<Vec<_>>();

    let job_id = api
        .submit_request(&package_type, &packages, false, project, None, group.map(String::from))
        .await?;

    Ok(job_id)
}

/// Retrieve user info.
/// Equivalent to `phylum auth status`.
#[op]
async fn get_user_info(op_state: Rc<RefCell<OpState>>) -> Result<UserInfo> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    api.user_info().await.map_err(Error::from)
}

/// Retrieve the access token.
/// Equivalent to `phylum auth token --bearer`.
#[op]
async fn get_access_token(
    op_state: Rc<RefCell<OpState>>,
    ignore_certs: bool,
) -> Result<AccessToken> {
    let refresh_token = get_refresh_token::call(op_state.clone()).await?;

    let state = ExtensionState::from(op_state);
    let api = state.api().await?;
    let config = api.config();

    let access_token =
        crate::auth::handle_refresh_tokens(&refresh_token, ignore_certs, &config.connection.uri)
            .await?
            .access_token;
    Ok(access_token)
}

/// Retrieve the refresh token.
/// Equivalent to `phylum auth token`.
#[op]
async fn get_refresh_token(op_state: Rc<RefCell<OpState>>) -> Result<RefreshToken> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;
    let config = api.config();

    config
        .auth_info
        .offline_access
        .clone()
        .ok_or_else(|| anyhow!("User is not currently authenticated"))
}

/// Retrieve a job's status.
/// Equivalent to `phylum history job`.
#[op]
async fn get_job_status(
    op_state: Rc<RefCell<OpState>>,
    job_id: String,
) -> Result<JobStatusResponse<PackageStatusExtended>> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let job_id = JobId::from_str(&job_id)?;
    api.get_job_status_ext(&job_id).await.map_err(Error::from)
}

/// Retrieve a project's details.
/// Equivalent to `phylum history project`.
#[op]
async fn get_project_details(
    op_state: Rc<RefCell<OpState>>,
    project_name: Option<String>,
) -> Result<ProjectDetailsResponse> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let project_name = project_name.map(String::from).map(Result::Ok).unwrap_or_else(|| {
        get_current_project()
            .map(|p| p.name)
            .ok_or_else(|| anyhow!("Failed to find a valid project configuration"))
    })?;
    api.get_project_details(&project_name).await.map_err(Error::from)
}

/// Analyze a single package.
/// Equivalent to `phylum package`.
#[op]
async fn get_package_details(
    op_state: Rc<RefCell<OpState>>,
    name: String,
    version: String,
    package_type: String,
) -> Result<Package> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let package_type = PackageType::from_str(&package_type)
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
#[op]
async fn parse_lockfile(
    _op_state: Rc<RefCell<OpState>>,
    lockfile: String,
    lockfile_type: Option<String>,
) -> Result<PackageLock> {
    // TODO: Validate read permissions
    // Ensure extension has file read-access.
    // let state = ExtensionState::from(op_state);
    // state.permissions.read.validate(&lockfile, "read")?;

    // Fallback to automatic parser without lockfile type specified.
    let lockfile_type = match lockfile_type {
        Some(lockfile_type) => lockfile_type,
        None => {
            let (packages, package_type) = parse::get_packages_from_lockfile(Path::new(&lockfile))?;
            return Ok(PackageLock {
                package_type,
                packages: packages.into_iter().map(PackageSpecifier::from).collect(),
            });
        },
    };

    // Attempt to parse as requested lockfile type.

    let parser = LOCKFILE_PARSERS
        .iter()
        .find_map(|(name, parser)| (*name == lockfile_type).then(|| *parser))
        .ok_or_else(|| anyhow!("Unrecognized lockfile type: `{lockfile_type}`"))?;

    let lockfile_data = fs::read_to_string(&lockfile)
        .await
        .with_context(|| format!("Could not read lockfile at '{lockfile}'"))?;
    let packages = parser.parse(&lockfile_data)?;

    Ok(PackageLock {
        package_type: parser.package_type(),
        packages: packages.into_iter().map(PackageSpecifier::from).collect(),
    })
}

pub(crate) fn api_decls() -> Vec<OpDecl> {
    vec![
        analyze::decl(),
        get_user_info::decl(),
        get_access_token::decl(),
        get_refresh_token::decl(),
        get_job_status::decl(),
        get_project_details::decl(),
        get_package_details::decl(),
        parse_lockfile::decl(),
    ]
}
