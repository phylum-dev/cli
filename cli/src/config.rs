use std::env::VarError;
#[cfg(unix)]
use std::fs::{DirBuilder, Permissions};
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    pub offline_access: Option<RefreshToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub connection: ConnectionInfo,
    pub auth_info: AuthInfo,
    pub request_type: PackageType,
    pub last_update: Option<usize>,
    #[serde(deserialize_with = "default_option_bool")]
    pub ignore_certs: bool,
}

fn default_option_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: ProjectId,
    pub name: String,
    pub created_at: DateTime<Local>,
    pub group_name: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connection: ConnectionInfo { uri: "https://api.phylum.io".into() },
            auth_info: AuthInfo { offline_access: None },
            request_type: PackageType::Npm,
            ignore_certs: false,
            last_update: None,
        }
    }
}

/// Create or open a file. If the file is created, it will restrict permissions
/// to allow read/write access only to the current user.
fn create_private_file<P: AsRef<Path>>(path: P) -> io::Result<fs::File> {
    // Use OpenOptions so that we can specify the permission bits
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        opts.mode(0o600);
    }

    // Windows file permissions are complicated and home folders aren't usually
    // globally readable, so we can ignore Windows for now.

    opts.open(path)
}

// TODO: define explicit error types
// TODO: This is NOT atomic, and file corruption can occur
// TODO: Config should be saved to temp file first, then rename() used to 'move'
// it to new location Rename is guaranteed atomic. Need to handle case when
// files are on different mount point
pub fn save_config<T>(path: &Path, config: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(config_dir) = path.parent() {
        #[cfg(not(unix))]
        fs::create_dir_all(config_dir)?;

        #[cfg(unix)]
        DirBuilder::new().recursive(true).mode(0o700).create(config_dir)?;
    }
    let yaml = serde_yaml::to_string(config)?;

    create_private_file(path)?.write_all(yaml.as_ref())?;

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

    // If an api token has been set in the environment, prefer that
    match env::var("PHYLUM_API_KEY") {
        Ok(key) if !key.is_empty() => {
            config.auth_info.offline_access = Some(RefreshToken::new(key));
        },
        Ok(_) => {
            print_user_warning!("Ignoring empty PHYLUM_API_KEY");
        },
        Err(VarError::NotUnicode(_)) => {
            print_user_warning!("Ignoring invalid PHYLUM_API_KEY");
        },
        Err(VarError::NotPresent) => {},
    }

    // The code that checks if we have a token expects `Some(token)` to always be a
    // usable token.
    if config.auth_info.offline_access.as_ref().map(|t| t.as_str()) == Some("") {
        config.auth_info.offline_access = None;
    }

    Ok(config)
}

pub fn find_project_conf(starting_directory: &Path) -> Option<PathBuf> {
    let mut path: PathBuf = starting_directory.into();
    let mut attempts = 0;
    const MAX_DEPTH: u8 = 32;

    loop {
        let search_path = path.join(PROJ_CONF_FILE);
        if search_path.is_file() {
            return Some(search_path);
        }

        if attempts > MAX_DEPTH {
            return None;
        }
        path.push("..");
        attempts += 1;
    }
}

pub fn get_current_project() -> Option<ProjectConfig> {
    find_project_conf(Path::new(".")).and_then(|s| {
        log::info!("Found project configuration file at {}", s.to_string_lossy());
        parse_config(&s).ok()
    })
}

pub fn get_home_settings_path() -> Result<PathBuf> {
    let home_path =
        home::home_dir().ok_or_else(|| anyhow!("Couldn't find the user's home directory"))?;

    let config_path = dirs::config_dir()?.join("phylum").join("settings.yaml");
    let old_config_path = home_path.join(".phylum").join("settings.yaml");

    // Migrate the config from the old location.
    if !config_path.exists() && old_config_path.exists() {
        let config_dir = config_path.parent().unwrap();

        #[cfg(unix)]
        {
            fs::set_permissions(&old_config_path, Permissions::from_mode(0o600))?;
            DirBuilder::new().recursive(true).mode(0o700).create(&config_dir)?;
        }

        #[cfg(not(unix))]
        fs::create_dir_all(&config_dir)?;

        fs::rename(old_config_path, &config_path).unwrap();
    }

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    const CONFIG_TOKEN: &str = "FAKE TOKEN";

    fn write_test_config(path: &Path) {
        let con = ConnectionInfo { uri: "http://127.0.0.1".into() };

        let auth = AuthInfo { offline_access: Some(RefreshToken::new(CONFIG_TOKEN)) };

        let config = Config {
            connection: con,
            auth_info: auth,
            request_type: PackageType::Npm,
            ignore_certs: false,
            last_update: None,
        };
        save_config(path, &config).unwrap();
    }

    #[test]
    fn test_save_config() {
        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
    }

    #[test]
    fn test_parse_config() {
        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
        let config: Config = parse_config(tempfile.path()).unwrap();
        assert_eq!(config.request_type, PackageType::Npm);
    }

    #[test]
    fn test_pass_api_key_through_env() {
        const ENV_TOKEN: &str = "ENV VARIABLE TOKEN";

        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
        env::set_var("PHYLUM_API_KEY", ENV_TOKEN);

        let config: Config = read_configuration(tempfile.path()).unwrap();

        assert_eq!(config.auth_info.offline_access, Some(RefreshToken::new(ENV_TOKEN)));
    }

    #[test]
    fn test_ignore_empty_token() {
        let tempfile = NamedTempFile::new().unwrap();
        write_test_config(tempfile.path());
        env::set_var("PHYLUM_API_KEY", "");

        let config: Config = read_configuration(tempfile.path()).unwrap();

        assert_eq!(config.auth_info.offline_access, Some(RefreshToken::new(CONFIG_TOKEN)));
    }
}
