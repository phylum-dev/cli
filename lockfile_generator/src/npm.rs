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
        Ok(project_path.join("package-lock.json"))
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
