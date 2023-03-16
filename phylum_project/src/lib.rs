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
            created_at: Local::now(),
            lockfile_type: None,
            lockfile_path: None,
            lockfiles: Default::default(),
            root: Default::default(),
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
    let path = starting_directory.as_ref();
    // If `path` is like `.`, `path.parent()` is `None`.
    let canonicalized = path.canonicalize();
    let mut path = canonicalized.as_deref().ok().unwrap_or(path);

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

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempfile::TempDir;
    use uuid::uuid;

    use super::*;
    use crate::ProjectConfig;

    const PROJECT_ID: ProjectId = uuid!("a814bc7b-c17c-4e91-9515-edd7899680fb");
    const PROJECT_NAME: &str = "my project";
    const GROUP_NAME: &str = "my group";

    #[test]
    fn new_config_has_correct_lockfile_paths() {
        let mut config =
            ProjectConfig::new(PROJECT_ID, PROJECT_NAME.to_owned(), Some(GROUP_NAME.to_owned()));
        config.set_lockfiles(vec![LockfileConfig {
            path: PathBuf::from("Cargo.lock"),
            lockfile_type: "cargo".to_owned(),
        }]);

        let lockfiles = config.lockfiles();
        let [lockfile] = &lockfiles[..] else {
            panic!("Expected to get exactly one lockfile but got {lockfiles:?}");
        };

        assert_eq!(&PathBuf::from("Cargo.lock"), &lockfile.path);
    }

    #[test]
    fn deserialized_config_has_correct_lockfile_paths() {
        let mut config =
            ProjectConfig::new(PROJECT_ID, PROJECT_NAME.to_owned(), Some(GROUP_NAME.to_owned()));
        config.set_lockfiles(vec![LockfileConfig {
            path: PathBuf::from("Cargo.lock"),
            lockfile_type: "cargo".to_owned(),
        }]);

        let config: ProjectConfig =
            serde_yaml::from_str(&serde_yaml::to_string(&config).unwrap()).unwrap();

        let lockfiles = config.lockfiles();
        let [lockfile] = &lockfiles[..] else {
            panic!("Expected to get exactly one lockfile but got {lockfiles:?}");
        };

        assert_eq!(&PathBuf::from("Cargo.lock"), &lockfile.path);
    }

    #[test]
    fn when_root_set_has_correct_lockfile_paths() {
        let mut config =
            ProjectConfig::new(PROJECT_ID, PROJECT_NAME.to_owned(), Some(GROUP_NAME.to_owned()));
        config.set_lockfiles(vec![LockfileConfig {
            path: PathBuf::from("Cargo.lock"),
            lockfile_type: "cargo".to_owned(),
        }]);
        config.root = PathBuf::from("/home/user/project");

        let lockfiles = config.lockfiles();
        let [lockfile] = &lockfiles[..] else {
            panic!("Expected to get exactly one lockfile but got {lockfiles:?}");
        };

        assert_eq!(&PathBuf::from("/home/user/project/Cargo.lock"), &lockfile.path);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn find_project_conf_can_recurse_up() {
        // To verify the behavior of navigation to parent directories, we must construct
        // a filesystem where the parent directory cannot be reached by removing path
        // components.
        //
        //     temp:
        //       - r:
        //         - .phylum_project
        //         - 0:
        //           - 1:
        //             - 2: etc
        //       - cwd: link to temp/r/0/1/2/etc
        //
        // Starting from inside temp/cwd, the parent directory should be
        // temp/r/0/1/2/etc, not temp.
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("r");
        let cwd = temp.path().join("cwd");

        fs::create_dir_all(&root).unwrap();
        let file = root.join(PROJ_CONF_FILE);
        File::create(&file).unwrap();
        // This is 32 path components.
        // Use single characters for a better chance of not hitting path length
        // limitations.
        let subdir = root.join("0/1/2/3/4/5/6/7/8/9/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u");
        fs::create_dir_all(&subdir).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&subdir, &cwd).unwrap();
        // This only works on Windows if the user is an administrator or developer mode
        // is on.
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&subdir, &cwd).unwrap();

        let found = find_project_conf(&cwd, true);
        assert_eq!(
            Some(file.canonicalize().unwrap()),
            found.map(|f| f
                .canonicalize()
                .expect("Found configuration should be a canonicalizable path"))
        );
    }
}
