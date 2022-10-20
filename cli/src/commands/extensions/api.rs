//! Extension API functions.

use std::cell::RefCell;
#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
#[cfg(unix)]
use std::process;
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
use crate::commands::extensions::permissions;
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

/// File descriptors for Stdio.
#[cfg(unix)]
struct ProcessStdioFds {
    parent: Option<File>,
    child: Option<File>,
}

#[cfg(unix)]
impl TryFrom<ProcessStdio> for ProcessStdioFds {
    type Error = io::Error;

    fn try_from(stdio: ProcessStdio) -> io::Result<Self> {
        let (parent, child) = match stdio {
            ProcessStdio::Inherit => (None, None),
            ProcessStdio::Piped => unsafe {
                // Create a pipe to send STDIO from child to parent.
                let mut fds = [0, 0];
                if libc::pipe(fds.as_mut_ptr()) == -1 {
                    return Err(io::Error::last_os_error());
                }

                // Convert pipe FDs to Rust files.
                let rx = File::from_raw_fd(fds[0]);
                let tx = File::from_raw_fd(fds[1]);

                (Some(rx), Some(tx))
            },
            ProcessStdio::Null => {
                let file = OpenOptions::new().write(true).open("/dev/null")?;
                (None, Some(file))
            },
        };

        Ok(Self { parent, child })
    }
}

#[cfg(unix)]
impl ProcessStdioFds {
    /// Replace a FD with the child FD.
    fn replace_fd(&self, fd: RawFd) -> io::Result<()> {
        let child_fd = match &self.child {
            Some(child) => child.as_raw_fd(),
            None => return Ok(()),
        };

        if unsafe { libc::dup2(child_fd, fd) } == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
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
        let permissions = state.borrow_mut::<Permissions>();
        permissions.read.check(Path::new(&lockfile), None)?;
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
fn run_sandboxed(op_state: Rc<RefCell<OpState>>, process: Process) -> Result<ProcessOutput> {
    let Process { cmd, args, stdin, exceptions, .. } = process;

    let strict = exceptions.strict;
    let resolved_permissions = {
        let mut state = op_state.borrow_mut();
        let permissions = state.borrow_mut::<permissions::Permissions>();
        permissions::Permissions::from(exceptions).subset_of(permissions)
    }?;

    let mut stdout_fds: ProcessStdioFds = process.stdout.try_into()?;
    let mut stderr_fds: ProcessStdioFds = process.stderr.try_into()?;

    match unsafe { libc::fork() } {
        -1 => Err(io::Error::last_os_error().into()),
        // Handle child process.
        0 => {
            // Connect STDOUT/STDERR with parent if necessary.
            stdout_fds.replace_fd(libc::STDOUT_FILENO)?;
            stderr_fds.replace_fd(libc::STDERR_FILENO)?;

            // Apply sandboxing rules.
            lock_process(resolved_permissions, strict)?;

            // Setup process to be run.
            let mut command = Command::new(&cmd);
            command.args(&args);
            command.stdin(stdin);
            command.stdout(Stdio::inherit());
            command.stderr(Stdio::inherit());

            // Wait for process to complete.
            let status = command.status()?;

            // Terminate child process.
            if let Some(code) = status.code() {
                process::exit(code);
            } else if let Some(signal) = status.signal() {
                // Attempt to propagate kill signals.
                unsafe { libc::kill(process::id() as i32, signal) };

                // Fall back to arbitrary error.
                process::exit(113);
            } else {
                process::exit(0);
            }
        },
        // Handle parent process.
        child_pid => {
            // Drop write side of pipe FDs, so child can write to it.
            stdout_fds.child.take();
            stderr_fds.child.take();

            // Wait for the child to complete.
            let mut exit_code = 0;
            if unsafe { libc::waitpid(child_pid, (&mut exit_code) as *mut _, 0) } == -1 {
                return Err(io::Error::last_os_error().into());
            };

            // Check process exit status.
            let mut signal = None;
            let mut code = None;
            if libc::WIFEXITED(exit_code) {
                code = Some(libc::WEXITSTATUS(exit_code));
            } else if libc::WIFSIGNALED(exit_code) {
                signal = Some(libc::WTERMSIG(exit_code));
            } else if libc::WIFSTOPPED(exit_code) {
                signal = Some(libc::WSTOPSIG(exit_code));
            }

            // Read STDOUT/STDERR from pipe.
            let mut stdout = String::new();
            if let Some(mut stdout_fd) = stdout_fds.parent {
                stdout_fd.read_to_string(&mut stdout)?;
            }
            let mut stderr = String::new();
            if let Some(mut stderr_fd) = stderr_fds.parent {
                stderr_fd.read_to_string(&mut stderr)?;
            }

            Ok(ProcessOutput { stdout, stderr, success: code == Some(0), signal, code })
        },
    }
}

/// Lock down the current process.
#[cfg(unix)]
fn lock_process(exceptions: permissions::Permissions, strict: bool) -> Result<()> {
    let home_dir = dirs::home_dir()?;

    let mut birdcage = if strict { Birdcage::new()? } else { permissions::default_sandbox()? };

    // Apply filesystem exceptions.
    for path in exceptions.read.sandbox_paths().iter() {
        let path = dirs::expand_home_path(path, &home_dir);
        permissions::add_exception(&mut birdcage, Exception::Read(path))?;
    }
    for path in exceptions.write.sandbox_paths().iter() {
        let path = dirs::expand_home_path(path, &home_dir);
        permissions::add_exception(&mut birdcage, Exception::Write(path))?;
    }
    for path in exceptions.run.sandbox_paths().iter() {
        let path = dirs::expand_home_path(path, &home_dir);
        let absolute_path = permissions::resolve_bin_path(path);
        permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead(absolute_path))?;
    }

    // Apply network exceptions.
    if let permissions::Permission::Boolean(true) = exceptions.net {
        birdcage.add_exception(Exception::Networking)?;
    }

    // Apply environment variable exceptions.
    let env_exceptions = match &exceptions.env {
        Permission::Boolean(true) => vec![Exception::FullEnvironment],
        Permission::Boolean(false) => Vec::new(),
        Permission::List(keys) => {
            keys.iter().map(|key| Exception::Environment(key.clone())).collect()
        },
    };
    for exception in env_exceptions {
        birdcage.add_exception(exception)?;
    }

    // Lock down the process.
    birdcage.lock()?;

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
        op_permissions::decl(),
    ]
}
