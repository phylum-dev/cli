//! Go ecosystem.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Generator, Result};

pub struct Go;

impl Generator for Go {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        Ok(project_path.join("go.sum"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("go");
        command.args(["get", "-d"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Go"
    }

    fn check_prerequisites(&self, manifest_path: &Path) -> Result<()> {
        if manifest_path.file_name() != Some(OsStr::new("go.mod")) {
            Err(Error::InvalidManifest(manifest_path.to_path_buf()))
        } else {
            Ok(())
        }
    }
}
