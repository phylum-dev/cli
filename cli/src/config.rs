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
use phylum_project::{LockfileConfig, ProjectConfig};
use phylum_types::types::auth::RefreshToken;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{dirs, print_user_warning};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionInfo {
    pub uri: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub connection: ConnectionInfo,
    pub auth_info: AuthInfo,
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

/// Get lockfiles from CLI, falling back to the current project when missing.
pub fn lockfiles(
    matches: &clap::ArgMatches,
    project: Option<&ProjectConfig>,
) -> Result<Vec<LockfileConfig>> {
    let cli_lockfile_type = matches.try_get_one::<String>("lockfile-type").unwrap_or(None);
    let cli_lockfiles = matches.get_many::<String>("lockfile");

    match cli_lockfiles {
        Some(cli_lockfiles) => {
            let lockfile_type = cli_lockfile_type.cloned().unwrap_or_else(|| "auto".into());
            Ok(cli_lockfiles
                .map(|lockfile| LockfileConfig::new(lockfile, lockfile_type.clone()))
                .collect())
        },
        None => {
            // Try the project file first.
            let project_lockfiles = project.map(|project| project.lockfiles());

            // Fallback to walking the directory.
            let lockfiles = project_lockfiles.unwrap_or_else(|| find_lockfiles("."));

            // Ask for explicit lockfile if none were found.
            if lockfiles.is_empty() {
                return Err(anyhow!("Missing lockfile parameter"));
            }

            Ok(lockfiles)
        },
    }
}

/// Find lockfiles at or below the specified directory.
pub fn find_lockfiles(directory: impl AsRef<Path>) -> Vec<LockfileConfig> {
    phylum_lockfile::find_lockfiles_at(directory)
        .drain(..)
        .map(|(path, format)| LockfileConfig::new(path, format.to_string()))
        .collect()
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
    const LOCALHOST: &str = "http://127.0.0.1";

    fn test_config() -> Config {
        Config {
            connection: ConnectionInfo { uri: String::from(LOCALHOST) },
            auth_info: AuthInfo {
                offline_access: Some(RefreshToken::new(CONFIG_TOKEN)),
                env_token: Some(RefreshToken::new(ENV_TOKEN)),
            },
            ignore_certs_cli: false,
            ignore_certs: false,
            last_update: None,
        }
    }

    fn write_test_config(path: &Path) {
        let config = test_config();
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

        let mut orig_config = test_config();
        // Clearing env token is expected.
        orig_config.auth_info.env_token = None;

        assert_eq!(config, orig_config);
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
