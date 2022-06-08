use std::cell::RefCell;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::str::FromStr;

use crate::commands::parse::{get_packages_from_lockfile, LOCKFILE_PARSERS};
use crate::config::get_current_project;
use crate::{api::PhylumApi, auth::UserInfo};

use anyhow::{anyhow, Context, Error, Result};
use deno_core::{op, OpDecl, OpState};
use futures::future::BoxFuture;
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;

/// Holds either an unawaited, boxed `Future`, or the result of awaiting the future.
pub(crate) enum OnceFuture<T: Unpin> {
    Future(BoxFuture<'static, T>),
    Awaited(T),
}

impl<T: Unpin> OnceFuture<T> {
    fn new(inner: BoxFuture<'static, T>) -> Self {
        OnceFuture::Future(inner)
    }

    async fn get(&mut self) -> &T {
        match *self {
            OnceFuture::Future(ref mut inner) => {
                *self = OnceFuture::Awaited(inner.await);
                match *self {
                    OnceFuture::Future(..) => unreachable!(),
                    OnceFuture::Awaited(ref mut inner) => inner,
                }
            }
            OnceFuture::Awaited(ref mut inner) => inner,
        }
    }
}

/// Opaquely encapsulates the extension state.
pub struct ExtensionState(OnceFuture<Result<PhylumApi>>);

impl ExtensionState {
    /// Borrow an instance of `ExtensionState` from a reference to Deno's
    /// `OpState` state handler, and retrieve a reference to the inner state.
    /// The references lives as long as `&mut OpState`.
    async fn borrow_from(state: &mut OpState) -> Result<&PhylumApi> {
        state
            .borrow_mut::<ExtensionState>()
            .0
            .get()
            .await
            .as_ref()
            .map_err(|e| anyhow!("{:?}", e))
    }
}

impl From<BoxFuture<'static, Result<PhylumApi>>> for ExtensionState {
    fn from(t: BoxFuture<'static, Result<PhylumApi>>) -> Self {
        Self(OnceFuture::new(t))
    }
}

//
// Extension API functions
// These functions need not be public, as Deno's declarations (`::decl()`) cloak
// them in a data structure that is consumed by the runtime extension builder.
//

/// Analyze a lockfile.
/// Equivalent to `phylum analyze`.
#[op]
async fn analyze(
    state: Rc<RefCell<OpState>>,
    lockfile: &str,
    project: Option<&str>,
    group: Option<&str>,
) -> Result<ProjectId> {
    // The general pattern for accessing state in extensions API functions is:
    // - borrow from Deno's state handler
    // - pin the borrowed reference guard
    // - extract `Result<&PhylumApi>` from ExtensionState
    let mut state = Pin::new(state.borrow_mut());
    let api = ExtensionState::borrow_from(&mut state).await?;

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
#[op]
async fn get_user_info(state: Rc<RefCell<OpState>>) -> Result<UserInfo> {
    let mut state = Pin::new(state.borrow_mut());
    let api = ExtensionState::borrow_from(&mut state).await?;

    api.user_info().await.map_err(Error::from)
}

/// Retrieve the access token.
/// Equivalent to `phylum auth token --bearer`.
#[op]
async fn get_access_token(state: Rc<RefCell<OpState>>, ignore_certs: bool) -> Result<AccessToken> {
    let refresh_token = get_refresh_token::call(state.clone()).await?;
    let mut state = Pin::new(state.borrow_mut());
    let config = ExtensionState::borrow_from(&mut state).await?.config();

    let access_token =
        crate::auth::handle_refresh_tokens(&refresh_token, ignore_certs, &config.connection.uri)
            .await?
            .access_token;
    Ok(access_token)
}

/// Retrieve the refresh token.
/// Equivalent to `phylum auth token`.
#[op]
async fn get_refresh_token(state: Rc<RefCell<OpState>>) -> Result<RefreshToken> {
    let mut state = Pin::new(state.borrow_mut());
    let config = ExtensionState::borrow_from(&mut state).await?.config();

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
    state: Rc<RefCell<OpState>>,
    job_id: Option<&str>,
) -> Result<JobStatusResponse<PackageStatusExtended>> {
    let mut state = Pin::new(state.borrow_mut());
    let api = ExtensionState::borrow_from(&mut state).await?;

    let job_id = job_id
        .map(|job_id| JobId::from_str(job_id).ok())
        .unwrap_or_else(|| get_current_project().map(|p| p.id))
        .ok_or_else(|| anyhow!("Failed to find a valid project configuration"))?;
    api.get_job_status_ext(&job_id).await.map_err(Error::from)
}

/// Retrieve a project's details.
/// Equivalent to `phylum history project`.
#[op]
async fn get_project_details(
    state: Rc<RefCell<OpState>>,
    project_name: Option<&str>,
) -> Result<ProjectDetailsResponse> {
    let mut state = Pin::new(state.borrow_mut());
    let api = ExtensionState::borrow_from(&mut state).await?;

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
#[op]
async fn analyze_package(
    state: Rc<RefCell<OpState>>,
    name: &str,
    version: &str,
    package_type: &str,
) -> Result<Package> {
    let mut state = Pin::new(state.borrow_mut());
    let api = ExtensionState::borrow_from(&mut state).await?;

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
#[op]
fn parse_lockfile(lockfile: &str, lockfile_type: &str) -> Result<Vec<PackageDescriptor>> {
    let parser = LOCKFILE_PARSERS
        .iter()
        .find_map(|(name, parser)| (*name == lockfile_type).then(|| *parser))
        .ok_or_else(|| anyhow!("Unrecognized lockfile type: `{lockfile_type}`"))?;

    parser.parse_file(Path::new(lockfile))
}

pub(crate) fn api_decls() -> Vec<OpDecl> {
    vec![
        analyze::decl(),
        get_user_info::decl(),
        get_access_token::decl(),
        get_refresh_token::decl(),
        get_job_status::decl(),
        get_project_details::decl(),
        analyze_package::decl(),
        parse_lockfile::decl(),
    ]
}
