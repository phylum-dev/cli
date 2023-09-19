//! JavaScript npm ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Generator, Result};

pub struct Npm;

impl Generator for Npm {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        let output = Command::new("npm").current_dir(project_path).args(["root"]).output()?;

        // Ensure command was successful.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::NonZeroExit(output.status.code(), stderr.into()));
        }

        // Parse root node_modules output.
        let stdout = String::from_utf8(output.stdout).map_err(Error::InvalidUtf8)?;
        let workspace_root = stdout
            .strip_suffix("/node_modules\n")
            .ok_or_else(|| Error::UnexpectedOutput("npm root", stdout.clone()))?;

        Ok(PathBuf::from(workspace_root).join("package-lock.json"))
    }

    fn conflicting_files(&self, manifest_path: &Path) -> Result<Vec<PathBuf>> {
        Ok(vec![
            self.lockfile_path(manifest_path)?,
            PathBuf::from("npm-shrinkwrap.json"),
            PathBuf::from("yarn.lock"),
        ])
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("npm");
        command.args(["install", "--package-lock-only", "--ignore-scripts"]);
        command
    }

    fn tool(&self) -> &'static str {
        "npm"
    }
}
