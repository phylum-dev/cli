//! Java maven ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Generator, Result};

pub struct Maven;

#[cfg(not(windows))]
fn maven_command() -> Command {
    Command::new("mvn")
}

#[cfg(windows)]
fn maven_command() -> Command {
    // Maven uses a batch script on Windows
    Command::new("mvn.cmd")
}

impl Generator for Maven {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        Ok(project_path.join("effective-pom.xml"))
    }

    fn command(&self, manifest_path: &Path) -> Command {
        let lockfile_path = self.lockfile_path(manifest_path).unwrap();
        let mut command = maven_command();
        command.args([
            "-q",
            "help:effective-pom",
            &format!("-Doutput={}", lockfile_path.display()),
        ]);
        command
    }

    fn tool(&self) -> &'static str {
        "Maven"
    }
}
