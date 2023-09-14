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
            .args(["metadata", &manifest_arg, "--format-version=1"])
            .output()?;

        // Ensure command was successful.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::NonZeroExit(output.status.code(), stderr.into()));
        }

        // Parse metadata output.
        let stdout = String::from_utf8(output.stdout).map_err(Error::InvalidUtf8)?;
        let metadata: Metadata = serde_json::from_str(&stdout)?;

        Ok(PathBuf::from(metadata.workspace_root).join("Cargo.lock"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("cargo");
        command.args(["generate-lockfile"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Cargo"
    }
}

/// Output of `cargo metadata`.
#[derive(Deserialize)]
struct Metadata {
    workspace_root: String,
}
