use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use phylum_types::types::auth::*;
use phylum_types::types::common::*;
use phylum_types::types::package::*;

pub const PROJ_CONF_FILE: &str = ".phylum_project";

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthInfo {
    pub oidc_discovery_url: Url,
    pub offline_access: Option<RefreshToken>,
}

pub type Packages = Vec<PackageDescriptor>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub connection: ConnectionInfo,
    pub auth_info: AuthInfo,
    pub request_type: PackageType,
    pub packages: Option<Packages>,
    pub last_update: Option<usize>,
    pub ignore_certs: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: ProjectId,
    pub name: String,
    pub created_at: DateTime<Local>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connection: ConnectionInfo {
                uri: "https://api.phylum.io".into(),
            },
            auth_info: AuthInfo {
                oidc_discovery_url:
                    "https://login.phylum.io/auth/realms/phylum/.well-known/openid-configuration"
                        .parse()
                        .unwrap(),
                offline_access: None,
            },
            request_type: PackageType::Npm,
            packages: None,
            last_update: None,
            ignore_certs: None,
        }
    }
}

/// Create or open a file. If the file is created, it will restrict permissions to allow read/write
/// access only to the current user.
fn create_private_file<P: AsRef<Path>>(path: P) -> io::Result<fs::File> {
    // Use OpenOptions so that we can specify the permission bits
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    // Windows file permissions are complicated and home folders aren't usually globally readable,
    // so we can ignore Windows for now.

    opts.open(path)
}

// TODO: define explicit error types
// TODO: This is NOT atomic, and file corruption can occur
// TODO: Config should be saved to temp file first, then rename() used to 'move' it to new location
// Rename is guaranteed atomic. Need to handle case when files are on different mount point
pub fn save_config<T>(path: &Path, config: &T) -> Result<()>
where
    T: Serialize,
{
    use std::io::Write;

    if let Some(config_dir) = path.parent() {
        fs::create_dir_all(config_dir)?
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
    if let Ok(key) = env::var("PHYLUM_API_KEY") {
        config.auth_info.offline_access = Some(RefreshToken::new(key));
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
        log::info!(
            "Found project configuration file at {}",
            s.to_string_lossy()
        );
        parse_config(&s).ok()
    })
}

pub fn get_home_settings_path() -> Result<PathBuf> {
    let home_path =
        home::home_dir().ok_or_else(|| anyhow!("Couldn't find the user's home directory"))?;

    Ok(home_path.join(".phylum").join("settings.yaml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn write_test_config() {
        let con = ConnectionInfo {
            uri: "http://127.0.0.1".into(),
        };

        let auth = AuthInfo {
            oidc_discovery_url: Url::parse("http://example.com").unwrap(),
            offline_access: Some(RefreshToken::new("FAKE TOKEN")),
        };

        let packages = vec![
            PackageDescriptor {
                name: "foo".into(),
                version: "1.2.3".into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "bar".into(),
                version: "3.4.5".into(),
                package_type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "baz".into(),
                version: "2020.2.12".into(),
                package_type: PackageType::Npm,
            },
        ];

        let config = Config {
            connection: con,
            auth_info: auth,
            request_type: PackageType::Npm,
            packages: Some(packages),
            last_update: None,
            ignore_certs: None,
        };
        let temp_dir = temp_dir();
        let test_config_file = temp_dir.as_path().join("test_config");
        save_config(&test_config_file, &config).unwrap();
    }

    #[test]
    fn test_save_config() {
        write_test_config();
    }

    #[test]
    fn test_parse_config() {
        write_test_config();
        let temp_dir = temp_dir();
        let test_config_file = temp_dir.as_path().join("test_config");
        let config: Config = parse_config(&test_config_file).unwrap();
        assert_eq!(config.request_type, PackageType::Npm);
    }

    #[test]
    fn test_pass_api_key_through_env() {
        write_test_config();
        let temp_dir = temp_dir();
        let test_config_file = temp_dir.as_path().join("test_config");
        env::set_var("PHYLUM_API_KEY", "ENV VARIABLE TOKEN");

        let config: Config = read_configuration(&test_config_file).unwrap();

        assert_eq!(
            String::from(&config.auth_info.offline_access.unwrap()),
            String::from("ENV VARIABLE TOKEN")
        );
    }
}
