use std::cell::RefCell;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{anyhow, Context, Error, Result};
use deno_runtime::deno_core::{op, OpDecl, OpState};
use futures::future::BoxFuture;
use tokio::sync::Mutex;

use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;

use crate::commands::parse::{get_packages_from_lockfile, LOCKFILE_PARSERS};
use crate::config::get_current_project;
use crate::{api::PhylumApi, auth::UserInfo};

/// Holds either an unawaited, boxed `Future`, or the result of awaiting the future.
enum OnceFuture<T: Unpin> {
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
pub struct ExtensionState(Mutex<OnceFuture<Result<Rc<PhylumApi>>>>);

impl From<BoxFuture<'static, Result<PhylumApi>>> for ExtensionState {
    fn from(extension_state_future: BoxFuture<'static, Result<PhylumApi>>) -> Self {
        Self(Mutex::new(OnceFuture::new(Box::pin(async {
            extension_state_future.await.map(Rc::new)
        }))))
    }
}

impl ExtensionState {
    async fn get(&self) -> Result<Rc<PhylumApi>> {
        // The mutex guard is only useful for synchronizing access to the encapsulated future.
        // Once a `Result<Rc<PhylumApi>>` is obtained, the guard is dropped: subsequent awaits on
        // `PhylumApi` methods are not synchronized via this mutex, and can happen concurrently.
        let mut guard = self.0.lock().await;
        Ok(Rc::clone(
            guard.get().await.as_ref().map_err(|e| anyhow!("{:?}", e))?,
        ))
    }
}

/// Wraps a shared, counted reference to the `PhylumApi` object.
///
/// The reference can be safely extracted from `Rc<RefCell<OpState>>` and
/// cloned; it will not require mutable access to the owning `RefCell`, so the
/// mutable borrow to it may be dropped.
struct ExtensionStateRef(Rc<PhylumApi>);

impl ExtensionStateRef {
    // This can not be implemented as the `From<T>` trait because of `async`.
    async fn from(state: Rc<RefCell<OpState>>) -> Result<ExtensionStateRef> {
        let state_ref = Pin::new(state.borrow());
        Ok(ExtensionStateRef(
            state_ref.borrow::<ExtensionState>().get().await?,
        ))
    }
}

impl Deref for ExtensionStateRef {
    type Target = PhylumApi;

    fn deref(&self) -> &Self::Target {
        &self.0
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
    let api = ExtensionStateRef::from(state).await?;

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
    let api = ExtensionStateRef::from(state).await?;

    api.user_info().await.map_err(Error::from)
}

/// Retrieve the access token.
/// Equivalent to `phylum auth token --bearer`.
#[op]
async fn get_access_token(state: Rc<RefCell<OpState>>, ignore_certs: bool) -> Result<AccessToken> {
    let refresh_token = get_refresh_token::call(state.clone()).await?;
    let api = ExtensionStateRef::from(state).await?;
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
async fn get_refresh_token(state: Rc<RefCell<OpState>>) -> Result<RefreshToken> {
    let api = ExtensionStateRef::from(state).await?;
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
    state: Rc<RefCell<OpState>>,
    job_id: Option<&str>,
) -> Result<JobStatusResponse<PackageStatusExtended>> {
    let api = ExtensionStateRef::from(state).await?;

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
    let api = ExtensionStateRef::from(state).await?;

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
async fn get_package_details(
    state: Rc<RefCell<OpState>>,
    name: &str,
    version: &str,
    package_type: &str,
) -> Result<Package> {
    let api = ExtensionStateRef::from(state).await?;

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

    let lockfile_data = std::fs::read_to_string(Path::new(lockfile))?;
    parser.parse(&lockfile_data)
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
