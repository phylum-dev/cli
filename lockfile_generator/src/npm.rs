//! JavaScript npm ecosystem.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use glob::Pattern;
use serde::Deserialize;

use crate::{Error, Generator, Result};

/// Maximum upwards travelsal when searching for a workspace root.
const WORKSPACE_ROOT_RECURSION_LIMIT: usize = 16;

pub struct Npm;

impl Generator for Npm {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let workspace_root = find_workspace_root(manifest_path)?;
        Ok(workspace_root.join("package-lock.json"))
    }

    fn conflicting_files(&self, manifest_path: &Path) -> Result<Vec<PathBuf>> {
        let workspace_root = find_workspace_root(manifest_path)?;
        Ok(vec![
            workspace_root.join("package-lock.json"),
            workspace_root.join("npm-shrinkwrap.json"),
            workspace_root.join("yarn.lock"),
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

/// Find the workspace root of an npm project.
pub(crate) fn find_workspace_root(manifest_path: impl AsRef<Path>) -> Result<PathBuf> {
    let manifest_path = manifest_path.as_ref();
    let original_root = manifest_path
        .parent()
        .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?
        .canonicalize()?;

    // Search parent directories for workspace manifests.
    let mut path = original_root.as_path();
    for _ in 0..WORKSPACE_ROOT_RECURSION_LIMIT {
        path = match path.parent() {
            Some(root_dir) => root_dir,
            None => break,
        };

        // Check if directory has an NPM manifest.
        let manifest_path = path.join("package.json");
        if !manifest_path.exists() {
            continue;
        }

        // Parse manifest.
        let content = fs::read_to_string(&manifest_path)?;
        let manifest: PackageJson = serde_json::from_str(&content)?;

        // Ignore non-workspace manifests.
        let workspaces = match manifest.workspaces {
            Some(workspaces) => workspaces,
            None => continue,
        };

        // Get original manifest's location relative to this manifest.
        let relative_path = original_root.strip_prefix(path)?;

        // Check if original manifest location matches any workspace glob.
        let is_root = workspaces.iter().any(|glob| {
            let glob = glob.strip_prefix("./").unwrap_or(glob);
            Pattern::new(glob).map_or(false, |pattern| pattern.matches_path(relative_path))
        });

        if is_root {
            return Ok(path.into());
        } else {
            return Ok(original_root);
        }
    }

    Ok(original_root)
}

/// Package JSON subset.
#[derive(Deserialize, Debug)]
struct PackageJson {
    workspaces: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NON_WORKSPACE_MANIFEST: &str = r#"{ "name": "test" }"#;

    #[test]
    fn root_with_workspace() {
        const WORKSPACE_MANIFEST: &str =
            r#"{ "name": "parent", "workspaces": ["./packages/sub/*"] }"#;

        // Write root workspace manifest.
        let tempdir = tempfile::tempdir().unwrap();
        let workspace_manifest = tempdir.path().join("package.json");
        fs::write(workspace_manifest, WORKSPACE_MANIFEST).unwrap();

        // Create irrelevant non-manifest directory.
        let nothing_dir = tempdir.path().join("packages");
        fs::create_dir_all(&nothing_dir).unwrap();

        // Write irrelevant intermediate manifest.
        let sub_dir = nothing_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();
        let sub_manifest = sub_dir.join("package.json");
        fs::write(sub_manifest, NON_WORKSPACE_MANIFEST).unwrap();

        // Write target project manifest.
        let project_dir = sub_dir.join("project");
        fs::create_dir_all(&project_dir).unwrap();
        let project_manifest = project_dir.join("package.json");
        fs::write(&project_manifest, NON_WORKSPACE_MANIFEST).unwrap();

        let root = find_workspace_root(&project_manifest).unwrap();
        assert_eq!(root, tempdir.path().to_path_buf().canonicalize().unwrap());
    }

    #[test]
    fn root_without_workspace() {
        let tempdir = tempfile::tempdir().unwrap();
        let manifest_path = tempdir.path().join("package.json");
        fs::write(&manifest_path, NON_WORKSPACE_MANIFEST).unwrap();

        let root = find_workspace_root(&manifest_path).unwrap();
        assert_eq!(root, tempdir.path().to_path_buf().canonicalize().unwrap());
    }
}
