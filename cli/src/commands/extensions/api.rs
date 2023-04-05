//! Extension API functions.

#[cfg(unix)]
use std::borrow::Cow;
use std::cell::RefCell;
#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
#[cfg(unix)]
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{anyhow, Error, Result};
use deno_runtime::deno_core::{op, OpDecl, OpState};
use deno_runtime::permissions::PermissionsContainer;
use phylum_lockfile::LockfileFormat;
use phylum_project::ProjectConfig;
use phylum_types::types::auth::{AccessToken, RefreshToken};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::group::ListUserGroupsResponse;
use phylum_types::types::package::{
    Package, PackageDescriptor, PackageSpecifier as PTPackageSpecifier, PackageSubmitResponse,
};
use phylum_types::types::project::ProjectSummaryResponse;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::api::{PhylumApiError, ResponseError};
use crate::auth::UserInfo;
use crate::commands::extensions::permissions::{self, Permission};
use crate::commands::extensions::state::ExtensionState;
use crate::commands::parse;
#[cfg(unix)]
use crate::commands::ExitCode;
#[cfg(unix)]
use crate::dirs;
use crate::types::PolicyEvaluationResponse;

/// Parsed lockfile content.
#[derive(Serialize, Deserialize, Debug)]
struct PackageLock {
    packages: Vec<PackageDescriptor>,
    format: LockfileFormat,
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
    env: Permission,
    #[serde(default)]
    net: bool,
    #[serde(default)]
    strict: bool,
}

#[cfg(unix)]
impl From<ProcessException> for permissions::Permissions {
    fn from(process_exception: ProcessException) -> Self {
        Self {
            read: process_exception.read,
            write: process_exception.write,
            run: process_exception.run,
            env: process_exception.env,
            net: Permission::Boolean(process_exception.net),
            unsandboxed_run: Permission::default(),
        }
    }
}

/// Standard I/O behavior.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
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
    packages: Vec<PackageDescriptor>,
    project: Option<String>,
    group: Option<String>,
) -> Result<JobId> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let (project, group) = match (project, group) {
        (Some(project), group) => (api.get_project_id(&project, group.as_deref()).await?, None),
        (None, _) => {
            if let Some(p) = phylum_project::get_current_project() {
                (p.id, p.group_name)
            } else {
                return Err(anyhow!("Failed to find a valid project configuration"));
            }
        },
    };

    let job_id = api.submit_request(&packages, project, None, group.map(String::from)).await?;

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
///
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
///
/// Equivalent to `phylum history job`.
#[op]
async fn get_job_status(
    op_state: Rc<RefCell<OpState>>,
    job_id: String,
    ignored_packages: Option<Vec<PackageDescriptor>>,
) -> Result<PolicyEvaluationResponse> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;

    let job_id = JobId::from_str(&job_id)?;
    let ignored_packages = ignored_packages.unwrap_or_default();
    let response = api.get_job_status(&job_id, ignored_packages).await?;

    Ok(response)
}

/// Show the user's currently linked project.
#[op]
fn get_current_project() -> Option<ProjectConfig> {
    phylum_project::get_current_project()
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

    api.submit_package(&PTPackageSpecifier {
        name: name.to_string(),
        version: version.to_string(),
        registry: package_type,
    })
    .await
    .map_err(Error::from)
    .and_then(|resp| match resp {
        PackageSubmitResponse::AlreadyProcessed(data) => Ok(data),
        _ => Err(anyhow!("Package has not yet been processed.")),
    })
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
        let permissions = state.borrow_mut::<PermissionsContainer>();
        permissions.check_read(Path::new(&lockfile), "phylum")?;
    }

    // Attempt to parse as requested lockfile type.
    let parsed = parse::parse_lockfile(lockfile, lockfile_type.as_deref())?;

    Ok(PackageLock { packages: parsed.packages, format: parsed.format })
}

/// Run a command inside a sandbox.
///
/// This runs the supplied command in a sandbox, without restricting the
/// permissions of the sandbox itself. As a result more privileged access is
/// possible even after the command has been spawned.
#[op]
#[cfg(unix)]
fn run_sandboxed(op_state: Rc<RefCell<OpState>>, process: Process) -> Result<ProcessOutput> {
    let Process { cmd, args, stdin, stdout, stderr, exceptions } = process;

    let strict = exceptions.strict;
    let state = ExtensionState::from(op_state);
    let resolved_permissions =
        permissions::Permissions::from(exceptions).subset_of(&state.extension().permissions())?;

    // Add sandbox subcommand argument.
    let mut sandbox_args = Vec::with_capacity(args.len());
    sandbox_args.push("sandbox".into());

    // Create CLI arguments for `phylum sandbox` permission exceptions.
    add_permission_args(&mut sandbox_args, &resolved_permissions, strict)?;

    // Add sandboxed command arguments.
    sandbox_args.push("--".into());
    sandbox_args.push(cmd.as_str().into());
    for arg in &args {
        sandbox_args.push(arg.into());
    }

    // Execute sandboxed command.
    let output = Command::new(env::current_exe()?)
        .args(sandbox_args.iter_mut().map(|arg| arg.to_mut()))
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .output()?;

    // Return explicit error when process start failed
    if output.status.code().map_or(false, |code| code == i32::from(&ExitCode::SandboxStart)) {
        return Err(anyhow!("Process {cmd:?} failed to start"));
    }

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
        signal: output.status.signal(),
        code: output.status.code(),
    })
}

/// Convert [permissions::Permissions] to arguments for `phylum sandbox`.
#[cfg(unix)]
fn add_permission_args<'a>(
    sandbox_args: &mut Vec<Cow<'a, str>>,
    permissions: &'a permissions::Permissions,
    strict: bool,
) -> Result<()> {
    if strict {
        sandbox_args.push("--strict".into());
    }

    // Add filesystem exception arguments.
    let home_dir = dirs::home_dir()?;
    for path in permissions.read.sandbox_paths().iter() {
        let path = dirs::expand_home_path(path, &home_dir);
        sandbox_args.push("--allow-read".into());
        sandbox_args.push(path.to_string_lossy().into_owned().into());
    }
    for path in permissions.write.sandbox_paths().iter() {
        let path = dirs::expand_home_path(path, &home_dir);
        sandbox_args.push("--allow-write".into());
        sandbox_args.push(path.to_string_lossy().into_owned().into());
    }
    for path in permissions.run.sandbox_paths().iter() {
        let path = dirs::expand_home_path(path, &home_dir);
        sandbox_args.push("--allow-run".into());
        sandbox_args.push(path.to_string_lossy().into_owned().into());
    }

    // Add network exception argument.
    if let permissions::Permission::Boolean(true) = permissions.net {
        sandbox_args.push("--allow-net".into());
    }

    // Add environment variable exception arguments.
    match &permissions.env {
        Permission::List(keys) => {
            // Filter out "*", since the CLI accepts this as allow-all.
            for key in keys.iter().filter(|key| key != &"*") {
                sandbox_args.push("--allow-env".into());
                sandbox_args.push(key.into());
            }
        },
        Permission::Boolean(true) => sandbox_args.push("--allow-env".into()),
        Permission::Boolean(false) => (),
    }

    Ok(())
}

/// Return error when trying to sandbox on Windows.
#[op]
#[cfg(not(unix))]
fn run_sandboxed(_process: Process) -> Result<ProcessOutput> {
    Err(anyhow!("Extension sandboxing is not supported on this platform"))
}

#[op]
fn op_permissions(op_state: Rc<RefCell<OpState>>) -> permissions::Permissions {
    let state = ExtensionState::from(op_state);
    state.extension().permissions().into_owned()
}

#[op]
async fn api_base_url(op_state: Rc<RefCell<OpState>>) -> Result<String> {
    let state = ExtensionState::from(op_state);
    let api = state.api().await?;
    let url = api.config().connection.uri.clone() + "/api";
    Ok(url)
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
        op_permissions::decl(),
        api_base_url::decl(),
    ]
}
