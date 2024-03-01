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
    #[serde(default, alias = "lockfiles")]
    depfiles: Vec<DepfileConfig>,
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
            depfiles: Default::default(),
            root: Default::default(),
        }
    }

    /// Get all dependency files of this project.
    pub fn depfiles(&self) -> Vec<DepfileConfig> {
        // Return new config format if present.
        if !self.depfiles.is_empty() {
            return self
                .depfiles
                .iter()
                .map(|depfile| {
                    let path = self.root.join(&depfile.path);
                    DepfileConfig::new(path, depfile.depfile_type.clone())
                })
                .collect();
        }

        // Fallback to old format.
        if let Some((path, lockfile_type)) =
            self.lockfile_path.as_ref().zip(self.lockfile_type.as_ref())
        {
            return vec![DepfileConfig::new(self.root.join(path), lockfile_type.clone())];
        }

        // Default to no dependency files.
        Vec::new()
    }

    /// Update the config's project.
    pub fn update_project(&mut self, project_id: ProjectId, name: String, group: Option<String>) {
        self.id = project_id;
        self.name = name;
        self.group_name = group;
    }

    /// Update the project's dependency files.
    pub fn set_depfiles(&mut self, depfiles: Vec<DepfileConfig>) {
        self.depfiles = depfiles;
    }

    /// Get project's root directory.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }
}

/// Dependency file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepfileConfig {
    pub path: PathBuf,
    #[serde(rename = "type")]
    pub depfile_type: String,
}

impl DepfileConfig {
    pub fn new(path: impl Into<PathBuf>, depfile_type: String) -> DepfileConfig {
        DepfileConfig { path: path.into(), depfile_type }
    }
}

/// Get current project configuration file's path.
pub fn find_project_conf(
    starting_directory: impl AsRef<Path>,
    recurse_upwards: bool,
) -> Option<PathBuf> {
    let max_depth = if recurse_upwards { 32 } else { 1 };
    // If `path` is like `.`, `path.parent()` is `None`.
    // Convert to a canonicalized path so removing components navigates up the
    // directory hierarchy.
    let mut path = starting_directory.as_ref().canonicalize().ok()?;

    for _ in 0..max_depth {
        let conf_path = path.join(PROJ_CONF_FILE);
        if conf_path.is_file() {
            return Some(conf_path);
        }

        if !path.pop() {
            return None;
        }
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

    const PROJECT_ID: ProjectId = uuid!("a814bc7b-c17c-4e91-9515-edd7899680fb");
    const PROJECT_NAME: &str = "my project";
    const GROUP_NAME: &str = "my group";

    #[test]
    fn new_config_has_correct_depfile_paths() {
        let mut config =
            ProjectConfig::new(PROJECT_ID, PROJECT_NAME.to_owned(), Some(GROUP_NAME.to_owned()));
        config.set_depfiles(vec![DepfileConfig {
            path: PathBuf::from("Cargo.lock"),
            depfile_type: "cargo".to_owned(),
        }]);

        let depfiles = config.depfiles();
        let [depfile] = &depfiles[..] else {
            panic!("Expected to get exactly one depfile but got {depfiles:?}");
        };

        assert_eq!(&PathBuf::from("Cargo.lock"), &depfile.path);
    }

    #[test]
    fn deserialized_config_has_correct_depfile_paths() {
        let mut config =
            ProjectConfig::new(PROJECT_ID, PROJECT_NAME.to_owned(), Some(GROUP_NAME.to_owned()));
        config.set_depfiles(vec![DepfileConfig {
            path: PathBuf::from("Cargo.lock"),
            depfile_type: "cargo".to_owned(),
        }]);

        let config: ProjectConfig =
            serde_yaml::from_str(&serde_yaml::to_string(&config).unwrap()).unwrap();

        let depfiles = config.depfiles();
        let [depfile] = &depfiles[..] else {
            panic!("Expected to get exactly one depfile but got {depfiles:?}");
        };

        assert_eq!(&PathBuf::from("Cargo.lock"), &depfile.path);
    }

    #[test]
    fn when_root_set_has_correct_depfile_paths() {
        let mut config =
            ProjectConfig::new(PROJECT_ID, PROJECT_NAME.to_owned(), Some(GROUP_NAME.to_owned()));
        config.set_depfiles(vec![DepfileConfig {
            path: PathBuf::from("Cargo.lock"),
            depfile_type: "cargo".to_owned(),
        }]);
        config.root = PathBuf::from("/home/user/project");

        let depfiles = config.depfiles();
        let [depfile] = &depfiles[..] else {
            panic!("Expected to get exactly one depfile but got {depfiles:?}");
        };

        assert_eq!(&PathBuf::from("/home/user/project/Cargo.lock"), &depfile.path);
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
