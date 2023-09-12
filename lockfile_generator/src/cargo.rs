//! Rust cargo ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_cargo::core;
use rust_cargo::util::config::Config;

use crate::{Error, Generator, Result};

pub struct Cargo;

impl Generator for Cargo {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let cargo_config = Config::default()?;
        let root_manifest =
            core::find_workspace_root(manifest_path, &cargo_config)?.unwrap_or_default();
        let workspace_root = root_manifest
            .parent()
            .ok_or_else(|| Error::InvalidManifest(root_manifest.to_path_buf()))?;
        Ok(workspace_root.join("Cargo.lock"))
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
