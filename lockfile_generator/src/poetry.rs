//! Python poetry ecosystem.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Generator, Result};

pub struct Poetry;

impl Generator for Poetry {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        Ok(project_path.join("poetry.lock"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("poetry");
        command.args(["lock"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Poetry"
    }

    fn check_prerequisites(&self, manifest_path: &Path) -> Result<()> {
        if manifest_path.file_name() != Some(OsStr::new("pyproject.toml")) {
            Err(Error::InvalidManifest(manifest_path.to_path_buf()))
        } else {
            Ok(())
        }
    }
}
