//! Java gradle ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Generator, Result};

pub struct Gradle;

impl Generator for Gradle {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        Ok(project_path.join("gradle.lockfile"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("gradle");
        command.args(["dependencies", "--write-locks"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Gradle"
    }
}
