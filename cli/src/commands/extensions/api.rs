//! Extension API functions.

use std::cell::RefCell;
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
#[cfg(unix)]
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{anyhow, Context, Error, Result};
#[cfg(unix)]
use birdcage::{Birdcage, Exception, Sandbox};
use deno_runtime::deno_core::{op, OpDecl, OpState};
use deno_runtime::permissions::Permissions;
use phylum_lockfile::LockfileFormat;
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::group::ListUserGroupsResponse;
use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageStatusExtended, PackageType,
};
use phylum_types::types::project::ProjectSummaryResponse;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::api::{PhylumApiError, ResponseError};
use crate::auth::UserInfo;
#[cfg(unix)]
use crate::commands::extensions::permissions as ext_permissions;
use crate::commands::extensions::permissions::Permission;
use crate::commands::extensions::state::ExtensionState;
use crate::commands::parse;
use crate::config::{self, ProjectConfig};
#[cfg(unix)]
use crate::dirs;

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

/// New process to be launched.
#[derive(Serialize, Deserialize, Debug)]
struct Process {
    cmd: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    stdin: ProcessStdio,
    #[serde(default)]
    stdout: ProcessStdio,
    #[serde(default)]
    stderr: ProcessStdio,
    #[serde(default)]
    exceptions: ProcessException,
}

/// Sandboxing exceptions.
#[derive(Serialize, Deserialize, Debug, Default)]
struct ProcessException {
    #[serde(default)]
    read: Permission,
    #[serde(default)]
    write: Permission,
    #[serde(default)]
    run: Permission,
    #[serde(default)]
    net: bool,
    #[serde(default)]
    strict: bool,
}

/// Standard I/O behavior.
#[derive(Serialize, Deserialize, Debug)]
enum ProcessStdio {
    #[serde(rename = "inherit")]
    Inherit,
    #[serde(rename = "piped")]
    Piped,
    #[serde(rename = "null")]
    Null,
}

impl Default for ProcessStdio {
    fn default() -> Self {
        Self::Inherit
    }
}

#[cfg(unix)]
impl From<ProcessStdio> for Stdio {
    fn from(stdio: ProcessStdio) -> Self {
        match stdio {
            ProcessStdio::Piped => Self::piped(),
            ProcessStdio::Inherit => Self::inherit(),
            ProcessStdio::Null => Self::null(),
        }
    }
}

/// Subprocess output.
#[derive(Serialize, Deserialize, Debug)]
struct ProcessOutput {
    stdout: String,
    stderr: String,
    success: bool,
    signal: Option<i32>,
    code: Option<i32>,
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
            if let Some(p) = config::get_current_project() {
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
        .offline_access()
        .cloned()
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

/// Show the user's currently linked project.
#[op]
fn get_current_project() -> Option<ProjectConfig> {
    config::get_current_project()
}

/// List all of the user's/group's project.
#[op]
async fn get_groups(op_state: Rc<RefCell<OpState>>) -> Result<ListUserGroupsResponse> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    api.get_groups_list().await.map_err(Error::from)
}

/// List all of the user's/group's project.
#[op]
async fn get_projects(
    op_state: Rc<RefCell<OpState>>,
    group: Option<String>,
) -> Result<Vec<ProjectSummaryResponse>> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    api.get_projects(group.as_deref()).await.map_err(Error::from)
}

#[derive(Serialize)]
struct CreatedProject {
    id: ProjectId,
    status: CreatedProjectStatus,
}

#[derive(Serialize)]
enum CreatedProjectStatus {
    Created,
    Exists,
}

/// Create a project.
#[op]
async fn create_project(
    op_state: Rc<RefCell<OpState>>,
    name: String,
    group: Option<String>,
) -> Result<CreatedProject> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    // Retrieve the id if the project already exists, otherwise return the id or the
    // error.
    match api.create_project(&name, group.as_deref()).await {
        Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => api
            .get_project_id(&name, group.as_deref())
            .await
            .map(|id| CreatedProject { id, status: CreatedProjectStatus::Exists })
            .map_err(|e| e.into()),
        Err(e) => Err(e.into()),
        Ok(id) => Ok(CreatedProject { id, status: CreatedProjectStatus::Created }),
    }
}

/// Delete a project.
#[op]
async fn delete_project(
    op_state: Rc<RefCell<OpState>>,
    name: String,
    group: Option<String>,
) -> Result<()> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let project_id = api.get_project_id(&name, group.as_deref()).await?;
    api.delete_project(project_id).await.map_err(|e| e.into())
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
    op_state: Rc<RefCell<OpState>>,
    lockfile: String,
    lockfile_type: Option<String>,
) -> Result<PackageLock> {
    // Ensure extension has file read-access.
    {
        let mut state = op_state.borrow_mut();
        state.borrow_mut::<Permissions>().read.check(Path::new(&lockfile), None)?;
    }

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

    let parser = lockfile_type
        .parse::<LockfileFormat>()
        .with_context(|| format!("Unrecognized lockfile type: `{lockfile_type}`"))?
        .parser();

    let lockfile_data = fs::read_to_string(&lockfile)
        .await
        .with_context(|| format!("Could not read lockfile at '{lockfile}'"))?;
    let packages = parser.parse(&lockfile_data)?;

    Ok(PackageLock {
        package_type: parser.package_type(),
        packages: packages.into_iter().map(PackageSpecifier::from).collect(),
    })
}

/// Run a command inside a sandbox.
///
/// This runs the supplied command in a sandbox, without restricting the
/// permissions of the sandbox itself. As a result more privileged access is
/// possible even after the command has been spawned.
#[op]
#[cfg(unix)]
fn run_sandboxed(process: Process) -> Result<ProcessOutput> {
    // Setup process to be run.
    let mut command = Command::new(&process.cmd);
    command.args(&process.args);
    command.stdin(process.stdin);
    command.stdout(process.stdout);
    command.stderr(process.stderr);

    // Apply sandbox to subprocess after fork.
    unsafe {
        command.pre_exec(move || {
            fn into_ioerr<E: Into<Box<dyn std::error::Error + Send + Sync>>>(err: E) -> io::Error {
                io::Error::new(io::ErrorKind::Other, err)
            }

            let home_dir = dirs::home_dir().map_err(into_ioerr)?;

            let mut birdcage = if process.exceptions.strict {
                Birdcage::new().map_err(into_ioerr)?
            } else {
                ext_permissions::default_sandbox().map_err(into_ioerr)?
            };

            for path in process.exceptions.read.sandbox_paths().iter() {
                let path = dirs::expand_home_path(path, &home_dir);
                ext_permissions::add_exception(&mut birdcage, Exception::Read(path))
                    .map_err(into_ioerr)?;
            }
            for path in process.exceptions.write.sandbox_paths().iter() {
                let path = dirs::expand_home_path(path, &home_dir);
                ext_permissions::add_exception(&mut birdcage, Exception::Write(path))
                    .map_err(into_ioerr)?;
            }
            for path in process.exceptions.run.sandbox_paths().iter() {
                let path = dirs::expand_home_path(path, &home_dir);
                let absolute_path = ext_permissions::resolve_bin_path(path);
                ext_permissions::add_exception(
                    &mut birdcage,
                    Exception::ExecuteAndRead(absolute_path),
                )
                .map_err(into_ioerr)?;
            }
            if process.exceptions.net {
                birdcage.add_exception(Exception::Networking).map_err(into_ioerr)?;
            }

            birdcage.lock().map_err(into_ioerr)?;
            Ok(())
        });
    }

    let output = command.output().with_context(|| {
        let cmd = process.cmd;
        let args = process.args.iter().map(|arg| format!("`{arg}`")).collect::<Vec<_>>().join(" ");
        format!("Executing sandboxed process failed: `{cmd}` {args}",)
    })?;

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
        code: output.status.code(),
        #[cfg(unix)]
        signal: output.status.signal(),
        #[cfg(not(unix))]
        signal: None,
    })
}

/// Return error when trying to sandbox on Windows.
#[op]
#[cfg(not(unix))]
fn run_sandboxed(_process: Process) -> Result<ProcessOutput> {
    Err(anyhow!("Extension sandboxing is not supported on this platform"))
}

#[op]
fn permissions(op_state: Rc<RefCell<OpState>>) -> ext_permissions::Permissions {
    let state = ExtensionState::from(op_state);
    (*state.extension().permissions()).clone()
}

pub(crate) fn api_decls() -> Vec<OpDecl> {
    vec![
        analyze::decl(),
        get_user_info::decl(),
        get_access_token::decl(),
        get_refresh_token::decl(),
        get_job_status::decl(),
        get_current_project::decl(),
        get_groups::decl(),
        get_projects::decl(),
        create_project::decl(),
        delete_project::decl(),
        get_package_details::decl(),
        parse_lockfile::decl(),
        run_sandboxed::decl(),
        permissions::decl(),
    ]
}
