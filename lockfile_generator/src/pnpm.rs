//! JavaScript pnpm ecosystem.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use crate::{Error, Generator, Result};

const WORKSPACE_MANIFEST_FILENAME: &str = "pnpm-workspace.yaml";
const WORKSPACE_DIR_ENV_VAR: &str = "NPM_CONFIG_WORKSPACE_DIR";

pub struct Pnpm;

impl Generator for Pnpm {
    // Based on PNPM's implementation:
    // https://github.com/pnpm/pnpm/blob/98377afd3452d92183e4b643a8b122887c0406c3/workspace/find-workspace-dir/src/index.ts
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        // Get project root from env variable.
        let workspace_dir_env = env::var_os(WORKSPACE_DIR_ENV_VAR)
            .or_else(|| env::var_os(WORKSPACE_DIR_ENV_VAR.to_lowercase()))
            .map(PathBuf::from);

        // Fallback to recursive search for `WORKSPACE_MANIFEST_FILENAME`.
        let workspace_root = workspace_dir_env.or_else(|| find_workspace_root(project_path));

        // Fallback to non-workspace location.
        let root = workspace_root.unwrap_or_default();

        Ok(root.join("pnpm-lock.yaml"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("pnpm");
        command.args(["install", "--lockfile-only", "--ignore-scripts"]);
        command
    }

    fn tool(&self) -> &'static str {
        "pnpm"
    }
}

/// Find PNPM workspace root.
fn find_workspace_root(mut path: &Path) -> Option<PathBuf> {
    loop {
        let dir = fs::read_dir(path).ok()?;

        for dir_entry in dir.into_iter().flatten().map(|entry| entry.path()) {
            if dir_entry.file_name().map_or(false, |name| name == WORKSPACE_MANIFEST_FILENAME) {
                return Some(path.into());
            }
        }

        path = path.parent()?;
    }
}
