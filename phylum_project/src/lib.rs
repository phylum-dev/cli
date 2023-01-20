//! Phylum project configuration handling.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use phylum_types::types::common::ProjectId;
use serde::{Deserialize, Serialize};

/// Project configuration file name.
pub const PROJ_CONF_FILE: &str = ".phylum_project";

/// Phylum project configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: ProjectId,
    pub name: String,
    pub created_at: DateTime<Local>,
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lockfile_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lockfile_path: Option<String>,
    #[serde(default)]
    lockfiles: Vec<LockfileConfig>,
    #[serde(skip)]
    root: PathBuf,
}

impl ProjectConfig {
    /// Create a new project configuration.
    pub fn new(id: ProjectId, name: String, group_name: Option<String>) -> Self {
        Self {
            group_name,
            name,
            id,
            root: PathBuf::from("."),
            created_at: Local::now(),
            lockfile_type: None,
            lockfile_path: None,
            lockfiles: Default::default(),
        }
    }

    /// Get all lockfiles of this project.
    pub fn lockfiles(&self) -> Vec<LockfileConfig> {
        // Return new lockfile format if present.
        if !self.lockfiles.is_empty() {
            return self
                .lockfiles
                .iter()
                .map(|lockfile| {
                    let path = self.root.join(&lockfile.path);
                    LockfileConfig::new(path, lockfile.lockfile_type.clone())
                })
                .collect();
        }

        // Fallback to old format.
        if let Some((path, lockfile_type)) =
            self.lockfile_path.as_ref().zip(self.lockfile_type.as_ref())
        {
            return vec![LockfileConfig::new(self.root.join(path), lockfile_type.clone())];
        }

        // Default to no lockfiles.
        Vec::new()
    }

    /// Update the project's lockfiles.
    pub fn set_lockfiles(&mut self, lockfiles: Vec<LockfileConfig>) {
        self.lockfiles = lockfiles;
    }
}

/// Lockfile metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileConfig {
    pub path: PathBuf,
    #[serde(rename = "type")]
    pub lockfile_type: String,
}

impl LockfileConfig {
    pub fn new(path: impl Into<PathBuf>, lockfile_type: String) -> LockfileConfig {
        LockfileConfig { path: path.into(), lockfile_type }
    }
}

/// Get current project configuration file's path.
pub fn find_project_conf(
    starting_directory: impl AsRef<Path>,
    recurse_upwards: bool,
) -> Option<PathBuf> {
    let max_depth = if recurse_upwards { 32 } else { 1 };
    let mut path = starting_directory.as_ref();

    for _ in 0..max_depth {
        let conf_path = path.join(PROJ_CONF_FILE);
        if conf_path.is_file() {
            return Some(conf_path);
        }

        path = path.parent()?;
    }

    if recurse_upwards {
        log::warn!("Max depth exceeded; abandoning search for .phylum_project file");
    }

    None
}

/// Get the current project's configuration.
pub fn get_current_project() -> Option<ProjectConfig> {
    find_project_conf(".", true).and_then(|config_path| {
        log::info!("Found project configuration file at {config_path:?}");
        let config_content = fs::read_to_string(&config_path).ok()?;
        let mut config: ProjectConfig = serde_yaml::from_str(&config_content).ok()?;
        config.root = config_path.parent()?.to_path_buf();
        Some(config)
    })
}
