//! JavaScript yarn ecosystem.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{npm, Error, Generator, Result};

pub struct Yarn;

impl Generator for Yarn {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let workspace_root = npm::find_workspace_root(manifest_path)?;
        Ok(workspace_root.join("yarn.lock"))
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
        if manifest_path.file_name() != Some(OsStr::new("package.json")) {
            return Err(Error::InvalidManifest(manifest_path.to_path_buf()));
        }

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
        Err(Error::NonZeroExit(output))
    }
}
