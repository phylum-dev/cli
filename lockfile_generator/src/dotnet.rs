//! C# .NET ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Generator, Result};

pub struct Dotnet;

impl Generator for Dotnet {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        Ok(project_path.join("packages.lock.json"))
    }

    fn command(&self, manifest_path: &Path) -> Command {
        let mut command = Command::new("dotnet");
        command.arg("restore").arg(manifest_path).arg("--use-lock-file");
        command
    }

    fn tool(&self) -> &'static str {
        ".NET"
    }
}
