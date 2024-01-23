//! Rust cargo ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::{Error, Generator, Result};

pub struct Cargo;

impl Generator for Cargo {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let manifest_arg = format!("--manifest-path={}", manifest_path.display());
        let output = Command::new("cargo")
            .args(["locate-project", &manifest_arg, "--workspace"])
            .output()?;

        // Ensure command was successful.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::NonZeroExit(output.status.code(), stderr.into()));
        }

        // Parse metadata output.
        let stdout = String::from_utf8(output.stdout).map_err(Error::InvalidUtf8)?;
        let project_location: ProjectLocation = serde_json::from_str(&stdout)?;

        // Go from root manifest to root project.
        let workspace_root = match project_location.root.parent() {
            Some(workspace_root) => workspace_root,
            None => return Err(Error::InvalidManifest(project_location.root.clone())),
        };

        Ok(workspace_root.join("Cargo.lock"))
    }

    fn command(&self, manifest_path: &Path) -> Command {
        let mut command = Command::new("cargo");
        command.arg("generate-lockfile").arg("--manifest-path").arg(manifest_path);
        command
    }

    fn tool(&self) -> &'static str {
        "Cargo"
    }
}

/// Output of `cargo locate-project`.
#[derive(Deserialize)]
struct ProjectLocation {
    root: PathBuf,
}
