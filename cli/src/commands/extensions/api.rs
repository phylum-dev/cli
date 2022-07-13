use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{anyhow, Context, Error, Result};
use deno_runtime::deno_core::{op, OpDecl, OpState};
use futures::future::BoxFuture;
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;
use tokio::fs;
use tokio::sync::Mutex;

use crate::api::PhylumApi;
use crate::auth::UserInfo;
use crate::commands::extensions::permissions::Permissions;
use crate::commands::parse::{self, get_packages_from_lockfile, LOCKFILE_PARSERS};
use crate::config::get_current_project;

/// Holds either an unawaited, boxed `Future`, or the result of awaiting the
/// future.
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
            },
            OnceFuture::Awaited(ref mut inner) => inner,
        }
    }
}

// XXX: Holding a mutable reference to any field inside `ExtensionState` across
// await points, will cause issues when that field is accessed from another
// extension API method.
//
// Accessing the `ExtensionState` is only safe through `ExtensionStateRef`,
// since that ensures that `OpState` is not accessed through a mutable reference
// which could be held across await points.
//
// When making a field of `ExtensionState` mutably accessible, the mutation
// should occur internally through SYNCHRONOUS methods (e.g. `with_x(|x| ...)`),
// so holding a mutable reference is impossible.
//
// If a mutable reference MUST be held across await points, like `PhylumApi`,
// its synchronization should force blocking through an async-safe `Mutex`. This
// will stall all extension API methods trying to access this field, so ensure
// the blocking duration is minimal.
//
/// Extension state the APIs have access to.
pub struct ExtensionState {
    api: Mutex<OnceFuture<Result<Rc<PhylumApi>>>>,
    permissions: Permissions,
}

impl ExtensionState {
    pub fn new(api: BoxFuture<'static, Result<PhylumApi>>, permissions: Permissions) -> Self {
        Self {
            permissions,
            api: Mutex::new(OnceFuture::new(Box::pin(async { api.await.map(Rc::new) }))),
        }
    }

    async fn api(&self) -> Result<Rc<PhylumApi>> {
        // The mutex guard is only useful for synchronizing internally mutable access to
        // the encapsulated future. Once a `Result<Rc<PhylumApi>>` is obtained,
        // the guard is dropped: subsequent awaits on `PhylumApi` methods are
        // not synchronized via this mutex, and can happen concurrently.
        let mut guard = self.api.lock().await;
        Ok(Rc::clone(guard.get().await.as_ref().map_err(|e| anyhow!("{:?}", e))?))
    }
}

/// Extension state reference.
///
/// This type allows easily getting an immutable reference to the extension
/// state stored in deno's [`OpState`].
struct ExtensionStateRef<'a>(Ref<'a, ExtensionState>);

impl<'a> ExtensionStateRef<'a> {
    fn from_op(op_state: &'a Rc<RefCell<OpState>>) -> Self {
        Self(Ref::map(op_state.borrow(), |op_state| op_state.borrow::<ExtensionState>()))
    }
}

impl<'a> Deref for ExtensionStateRef<'a> {
    type Target = ExtensionState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Extension API functions
// These functions need not be public, as Deno's declarations (`::decl()`) cloak
// them in a data structure that is consumed by the runtime extension builder.
//

/// Analyze a lockfile.
/// Equivalent to `phylum analyze`.
#[op]
async fn analyze(
    op_state: Rc<RefCell<OpState>>,
    lockfile: String,
    project: Option<String>,
    group: Option<String>,
) -> Result<ProjectId> {
    // Ensure extension has file read-access.
    let state = ExtensionStateRef::from_op(&op_state);
    state.permissions.read.validate(&lockfile, "read")?;

    let api = state.api().await?;

    let (packages, request_type) = get_packages_from_lockfile(Path::new(&lockfile))
        .context("Unable to locate any valid package in package lockfile")?;

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

    let job_id = api
        .submit_request(&request_type, &packages, false, project, None, group.map(String::from))
        .await?;

    Ok(job_id)
}

/// Retrieve user info.
/// Equivalent to `phylum auth status`.
#[op]
async fn get_user_info(op_state: Rc<RefCell<OpState>>) -> Result<UserInfo> {
    let state = ExtensionStateRef::from_op(&op_state);
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

    let state = ExtensionStateRef::from_op(&op_state);
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
    let state = ExtensionStateRef::from_op(&op_state);
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
    let state = ExtensionStateRef::from_op(&op_state);
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
    let state = ExtensionStateRef::from_op(&op_state);
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
    let state = ExtensionStateRef::from_op(&op_state);
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
    op_state: Rc<RefCell<OpState>>,
    lockfile: String,
    lockfile_type: Option<String>,
) -> Result<Vec<PackageDescriptor>> {
    // Ensure extension has file read-access.
    let state = ExtensionStateRef::from_op(&op_state);
    state.permissions.read.validate(&lockfile, "read")?;

    // Fallback to automatic parser without lockfile type specified.
    let lockfile_type = match lockfile_type {
        Some(lockfile_type) => lockfile_type,
        None => return Ok(parse::get_packages_from_lockfile(Path::new(&lockfile))?.0),
    };

    // Attempt to parse as requested lockfile type.

    let parser = LOCKFILE_PARSERS
        .iter()
        .find_map(|(name, parser)| (*name == lockfile_type).then(|| *parser))
        .ok_or_else(|| anyhow!("Unrecognized lockfile type: `{lockfile_type}`"))?;

    let lockfile_data = fs::read_to_string(&lockfile)
        .await
        .with_context(|| format!("Could not read lockfile at '{lockfile}'"))?;
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
