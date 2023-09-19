//! JavaScript yarn ecosystem.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::{Error, Generator, Result};

pub struct Yarn;

impl Generator for Yarn {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        let output = Command::new("yarn")
            .current_dir(project_path)
            .args(["workspaces", "list", "--json"])
            .output()?;

        // Ensure command was successful.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::NonZeroExit(output.status.code(), stderr.into()));
        }

        // Convert output to actual valid json.
        let stdout = String::from_utf8(output.stdout).map_err(Error::InvalidUtf8)?;
        let workspaces_json = format!("[{}]", stdout.trim_end().replace('\n', ","));

        // Parse workspace list.
        let mut workspaces: Vec<Workspace> = serde_json::from_str(&workspaces_json)?;

        // Sort by longest location path, to prefer longer matches.
        workspaces.sort_by(|a, b| b.location.len().cmp(&a.location.len()));

        // Find workspace root by stripping the first matching location path.
        let project_str = project_path
            .to_str()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        let workspace_root = workspaces
            .into_iter()
            .find_map(|project| project_str.strip_suffix(&project.location))
            .unwrap_or(project_str);

        Ok(PathBuf::from(workspace_root).join("yarn.lock"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("yarn");
        command.args(["install", "--mode=skip-build", "--mode=update-lockfile"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Yarn"
    }

    fn check_prerequisites(&self, manifest_path: &Path) -> Result<()> {
        let yarn_version = yarn_version(manifest_path)?;
        if yarn_version.starts_with("1.") {
            let version = yarn_version.trim().into();
            return Err(Error::UnsupportedCommandVersion("yarn", "2.0.0+", version));
        }

        Ok(())
    }
}

/// Get the yarn version of the project.
fn yarn_version(manifest_path: &Path) -> Result<String> {
    let canonicalized = fs::canonicalize(manifest_path)?;
    let project_path = canonicalized
        .parent()
        .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

    let output = Command::new("yarn").arg("--version").current_dir(project_path).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::NonZeroExit(output.status.code(), stderr.into()))
    }
}

/// Yarn workspace project.
#[derive(Deserialize)]
struct Workspace {
    location: String,
}
