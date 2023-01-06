use std::env::VarError;
#[cfg(not(unix))]
use std::fs::File;
#[cfg(unix)]
use std::fs::{DirBuilder, OpenOptions};
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use phylum_types::types::auth::RefreshToken;
use phylum_types::types::common::ProjectId;
use phylum_types::types::package::PackageType;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{dirs, print_user_warning};

pub const PROJ_CONF_FILE: &str = ".phylum_project";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub uri: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthInfo {
    offline_access: Option<RefreshToken>,
    #[serde(skip)]
    env_token: Option<RefreshToken>,
}

impl AuthInfo {
    pub fn new(offline_access: Option<RefreshToken>) -> Self {
        Self { offline_access, env_token: None }
    }

    pub fn offline_access(&self) -> Option<&RefreshToken> {
        let env_token = self.env_token.as_ref().filter(|token| !token.as_str().is_empty());
        let token = self.offline_access.as_ref().filter(|token| !token.as_str().is_empty());
        env_token.or(token)
    }

    pub fn set_offline_access(&mut self, offline_access: RefreshToken) {
        self.offline_access = Some(offline_access);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub connection: ConnectionInfo,
    pub auth_info: AuthInfo,
    pub request_type: PackageType,
    pub last_update: Option<usize>,
    #[serde(skip)]
    ignore_certs_cli: bool,
    #[serde(deserialize_with = "default_option_bool")]
    ignore_certs: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connection: ConnectionInfo { uri: "https://api.phylum.io".into() },
            auth_info: AuthInfo::default(),
            request_type: PackageType::Npm,
            ignore_certs_cli: false,
            ignore_certs: false,
            last_update: None,
        }
    }
}

impl Config {
    /// Check if certificates should be ignored.
    pub fn ignore_certs(&self) -> bool {
        self.ignore_certs_cli || self.ignore_certs
    }

    /// Set the CLI `--no-check-certificate` override value.
    pub fn set_ignore_certs_cli(&mut self, ignore_certs_cli: bool) {
        self.ignore_certs_cli = ignore_certs_cli;
    }
}

fn default_option_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileConfig {
    path: String,
    #[serde(rename = "type")]
    pub lockfile_type: String,
    #[serde(skip)]
    root: PathBuf,
}

impl LockfileConfig {
    pub fn new(path: String, lockfile_type: String) -> LockfileConfig {
        LockfileConfig { path, lockfile_type, root: PathBuf::from(".") }
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(&self.path)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: ProjectId,
    pub name: String,
    pub created_at: DateTime<Local>,
    pub group_name: Option<String>,
    #[serde(default)]
    pub lockfiles: Vec<LockfileConfig>,
    pub lockfile_type: Option<String>,
    lockfile_path: Option<String>,
    #[serde(skip)]
    root: PathBuf,
}

impl ProjectConfig {
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

    /// Get all lockfiles.
    pub fn lockfiles(&self) -> Vec<LockfileConfig> {
        let lockfile = match (&self.lockfile_path, &self.lockfile_type) {
            (Some(path), Some(lockfile_type)) => Some(LockfileConfig {
                path: path.clone(),
                lockfile_type: lockfile_type.clone(),
                root: self.root.clone(),
            }),
            _ => None,
        };
        lockfile
            .into_iter()
            .chain(
                self.lockfiles
                    .iter()
                    .cloned()
                    .map(|lockfile| LockfileConfig { root: self.root.clone(), ..lockfile }),
            )
            .collect()
    }

    /// Update the lockfile.
    pub fn set_lockfile(&mut self, lockfile_type: String, path: String) {
        self.lockfiles = vec![LockfileConfig { path, lockfile_type, root: self.root.clone() }];
    }
}

/// Atomically overwrite the configuration file.
#[cfg(unix)]
pub fn save_config<T>(path: &Path, config: &T) -> Result<()>
where
    T: Serialize,
{
    let yaml = serde_yaml::to_string(config)?;

    // Ensure config directory and its parents exist.
    let config_dir = path.parent().ok_or_else(|| anyhow!("config path is a directory"))?;
    DirBuilder::new().recursive(true).mode(0o700).create(config_dir)?;

    // Use target directory for temporary file path.
    //
    // It's not possible to create the file on tmpfs since the configuration file is
    // usually not on the same device, which causes `fs::rename` to fail.
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("invalid config name"))?;
    let tmp_path = config_dir.join(format!(".{file_name}.new"));

    // Create the temporary file for the new config.
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&tmp_path)?;

    // Write new config to the temporary file.
    file.write_all(yaml.as_bytes())?;

    // Atomically move the new config into place.
    fs::rename(tmp_path, path)?;

    Ok(())
}

/// Unatomically overwrite the configuration file.
#[cfg(not(unix))]
pub fn save_config<T>(path: &Path, config: &T) -> Result<()>
where
    T: Serialize,
{
    let yaml = serde_yaml::to_string(config)?;

    // Ensure config directory and its parents exist.
    let config_dir = path.parent().ok_or_else(|| anyhow!("config path is a directory"))?;
    fs::create_dir_all(config_dir)?;

    // Write new configuration to the file.
    let mut file = File::create(path)?;
    file.write_all(yaml.as_bytes())?;

    Ok(())
}

pub fn parse_config<T>(path: &Path) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let contents = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str::<T>(&contents)?)
}

pub fn read_configuration(path: &Path) -> Result<Config> {
    let mut config: Config = match parse_config(path) {
        Ok(c) => c,
        Err(orig_err) => match orig_err.downcast_ref::<io::Error>() {
            Some(e) if e.kind() == io::ErrorKind::NotFound => Config::default(),
            _ => return Err(orig_err),
        },
    };

    // Store API token set in environment.
    match env::var("PHYLUM_API_KEY") {
        Ok(key) if !key.is_empty() => {
            config.auth_info.env_token = Some(RefreshToken::new(key));
        },
        Ok(_) => print_user_warning!("Ignoring empty PHYLUM_API_KEY"),
        Err(VarError::NotUnicode(_)) => print_user_warning!("Ignoring invalid PHYLUM_API_KEY"),
        Err(VarError::NotPresent) => (),
    }

    Ok(config)
}

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

pub fn get_current_project() -> Option<ProjectConfig> {
    find_project_conf(".", true).and_then(|config_path| {
        log::info!("Found project configuration file at {config_path:?}");
        let mut config: ProjectConfig = parse_config(&config_path).ok()?;
        config.root = config_path.parent()?.to_path_buf();
        Some(config)
    })
}

pub fn get_home_settings_path() -> Result<PathBuf> {
    let config_path = dirs::config_dir()?.join("phylum").join("settings.yaml");
    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    const CONFIG_TOKEN: &str = "FAKE TOKEN";
    const ENV_TOKEN: &str = "ENV TOKEN";

    fn write_test_config(path: &Path) {
        let con = ConnectionInfo { uri: "http://127.0.0.1".into() };

        let auth = AuthInfo {
            offline_access: Some(RefreshToken::new(CONFIG_TOKEN)),
            env_token: Some(RefreshToken::new(ENV_TOKEN)),
        };

        let config = Config {
            connection: con,
            auth_info: auth,
            request_type: PackageType::Npm,
            ignore_certs_cli: false,
            ignore_certs: false,
            last_update: None,
        };
        save_config(path, &config).unwrap();
    }

    #[test]
    fn write_config_works() {
        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
    }

    #[test]
    fn write_parses_identical() {
        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
        let config: Config = parse_config(tempfile.path()).unwrap();
        assert_eq!(config.request_type, PackageType::Npm);
    }

    #[test]
    fn write_ignores_env() {
        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
        let config: Config = parse_config(tempfile.path()).unwrap();
        assert_eq!(config.auth_info.offline_access(), Some(&RefreshToken::new(CONFIG_TOKEN)));
        assert_eq!(config.auth_info.env_token, None);
    }

    #[test]
    fn prefer_env_token() {
        let auth = AuthInfo {
            offline_access: Some(RefreshToken::new(CONFIG_TOKEN)),
            env_token: Some(RefreshToken::new(ENV_TOKEN)),
        };
        assert_eq!(auth.offline_access(), Some(&RefreshToken::new(ENV_TOKEN)));
    }
}
