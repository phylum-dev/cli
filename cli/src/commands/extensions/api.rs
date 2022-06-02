//! TODO remove the following annotation before merging. The functions in
//! this module should have as unique client the Deno runtime; pending its
//! implementation, having these functions unused would break CI.
#![allow(unused)]

use std::cell::RefCell;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use crate::commands::parse::{get_packages_from_lockfile, LOCKFILE_PARSERS};
use crate::config::{get_current_project, Config};
use crate::lockfiles::Parse;
use crate::{api::PhylumApi, auth::UserInfo};

use anyhow::{anyhow, Context, Error, Result};
use deno_core::parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use deno_core::{op, OpDecl, OpState};
use futures::future::BoxFuture;
use once_cell::sync::Lazy;
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectDetailsResponse;

// BROKEN

enum OnceFuture<T: Unpin> {
    Future(Option<BoxFuture<'static, Result<T>>>),
    Awaited(T),
}

impl<T: Unpin> OnceFuture<T> {
    fn new(t: BoxFuture<'static, T>) -> Self {
        OnceFuture::Future(Some(t))
    }

    async fn try_get(&mut self) -> Result<&T> {
        match *self {
            OnceFuture::Future(Some(ref mut t)) => {
                *self = OnceFuture::Awaited(t.await?);
                match *self {
                    OnceFuture::Future(..) => unreachable!(),
                    OnceFuture::Awaited(ref mut t) => Ok(t),
                }
            }
            OnceFuture::Awaited(ref mut t) => Ok(t)
        }
    }
}

type PhylumApiFut = OnceFuture<Result<PhylumApi>>;

/// Container for lazily evaluated dependencies of Extensions API functions. These values won't be
/// visible from extension code and will have to be set up by the JS runtime builder which will
/// load their configuration as any other command and then provide the pertinent factories.
pub(crate) struct InjectedDependencies {
    api: OnceFuture<Result<Arc<PhylumApi>>>,
    config: Config,
}

impl InjectedDependencies {
    pub(crate) async fn from_factories(
        api_factory: BoxFuture<'static, Result<Arc<PhylumApi>>>,
        config: Config,
    ) -> Self {
        InjectedDependencies {
            api: OnceFuture::new(api_factory),
            config,
        }
    }

    async fn api(&mut self) -> Result<Arc<PhylumApi>> {
        self.api
            .get()
            .await
            .as_ref()
            .cloned()
            .map_err(|e| anyhow!("{:?}", e))
    }
}

/// Analyze a lockfile.
/// Equivalent to `phylum analyze`.
#[op]
pub(crate) async fn analyze(
    state: Rc<RefCell<OpState>>,
    lockfile: &str,
    project: Option<&str>,
    group: Option<&str>,
) -> Result<ProjectId> {
    let mut state = Pin::new(state.borrow_mut());
    let api = state.borrow_mut::<PhylumApiFut>().get().await?;

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
pub(crate) async fn get_user_info(state: Rc<RefCell<OpState>>) -> Result<UserInfo> {
    let mut state = Pin::new(state.borrow_mut());
    let api = state.borrow_mut::<InjectedDependencies>().api().await?;
    api.user_info().await.map_err(Error::from)
}

/// Retrieve the access token.
/// Equivalent to `phylum auth token --bearer`.
#[op]
pub(crate) async fn get_access_token(
    state: Rc<RefCell<OpState>>,
    ignore_certs: bool,
) -> Result<AccessToken> {
    let refresh_token = get_refresh_token::call(state.clone())?;
    let mut state = Pin::new(state.borrow_mut());
    let config = &state.borrow_mut::<PhylumApiFut>().await?.config;

    let access_token =
        crate::auth::handle_refresh_tokens(&refresh_token, ignore_certs, &config.connection.uri)
            .await?
            .access_token;
    Ok(access_token)
}

/// Retrieve the refresh token.
/// Equivalent to `phylum auth token`.
#[op]
pub(crate) fn get_refresh_token(state: Rc<RefCell<OpState>>) -> Result<RefreshToken> {
    let mut state = state.borrow_mut();
    let config = &state.borrow_mut::<InjectedDependencies>().config;
    config
        .auth_info
        .offline_access
        .clone()
        .ok_or_else(|| anyhow!("User is not currently authenticated"))
}

/// Retrieve a job's status.
/// Equivalent to `phylum history job`.
#[op]
pub(crate) async fn get_job_status(
    state: Rc<RefCell<OpState>>,
    job_id: Option<&str>,
) -> Result<JobStatusResponse<PackageStatusExtended>> {
    let mut state = Pin::new(state.borrow_mut());
    let api = state.borrow_mut::<InjectedDependencies>().api().await?;

    let job_id = job_id
        .map(|job_id| JobId::from_str(job_id).ok())
        .unwrap_or_else(|| get_current_project().map(|p| p.id))
        .ok_or_else(|| anyhow!("Failed to find a valid project configuration"))?;
    api.get_job_status_ext(&job_id).await.map_err(Error::from)
}

/// Retrieve a project's details.
/// Equivalent to `phylum history project`.
#[op]
pub(crate) async fn get_project_details(
    state: Rc<RefCell<OpState>>,
    project_name: Option<&str>,
) -> Result<ProjectDetailsResponse> {
    let mut state = Pin::new(state.borrow_mut());
    let api = state.borrow_mut::<InjectedDependencies>().api().await?;

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
pub(crate) async fn analyze_package(
    state: Rc<RefCell<OpState>>,
    name: &str,
    version: &str,
    package_type: &str,
) -> Result<Package> {
    let mut state = Pin::new(state.borrow_mut());
    let api = state.borrow_mut::<InjectedDependencies>().api().await;

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
pub(crate) fn parse_lockfile(
    lockfile: &str,
    lockfile_type: &str,
) -> Result<Vec<PackageDescriptor>> {
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
