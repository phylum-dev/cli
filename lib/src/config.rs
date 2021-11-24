use chrono::{DateTime, Local};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::types::*;

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

// TODO: define explicit error types
// TODO: This is NOT atomic, and file corruption can occur
// TODO: Config should be saved to temp file first, then rename() used to 'move' it to new location
// Rename is guaranteed atomic. Need to handle case when files are on different mount point
pub fn save_config<T>(path: &str, config: &T) -> Result<(), Box<dyn Error + Send + Sync + 'static>>
where
    T: Serialize,
{
    let yaml = serde_yaml::to_string(config)?;
    fs::write(shellexpand::env(path)?.as_ref(), yaml)?;
    Ok(())
}

pub fn parse_config<T>(path: &str) -> Result<T, Box<dyn Error + Send + Sync + 'static>>
where
    T: serde::de::DeserializeOwned,
{
    let contents = fs::read_to_string(shellexpand::env(path)?.as_ref())?;
    let config: T = serde_yaml::from_str(&contents)?;
    Ok(config)
}

pub fn read_configuration(path: &str) -> Result<Config, Box<dyn Error + Send + Sync + 'static>> {
    let mut config: Config = parse_config(path)?;

    // If an api token has been set in the environment, prefer that
    if let Ok(key) = env::var("PHYLUM_API_KEY") {
        config.auth_info.offline_access = Some(RefreshToken::new(key));
    }
    Ok(config)
}

pub fn find_project_conf(starting_directory: &str) -> Option<String> {
    let mut path: PathBuf = starting_directory.into();
    let mut attempts = 0;
    const MAX_DEPTH: u8 = 32;

    loop {
        let search_path = path.join(PROJ_CONF_FILE);
        if search_path.is_file() {
            return Some(search_path.to_string_lossy().to_string());
        }

        if attempts > MAX_DEPTH {
            return None;
        }
        path.push("..");
        attempts += 1;
    }
}

pub fn get_current_project() -> Option<ProjectConfig> {
    find_project_conf(".").and_then(|s| {
        log::info!("Found project configuration file at {}", s);
        parse_config(&s).ok()
    })
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
                r#type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "bar".into(),
                version: "3.4.5".into(),
                r#type: PackageType::Npm,
            },
            PackageDescriptor {
                name: "baz".into(),
                version: "2020.2.12".into(),
                r#type: PackageType::Npm,
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
        save_config(test_config_file.to_str().unwrap(), &config).unwrap();
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
        let config: Config = parse_config(test_config_file.to_str().unwrap()).unwrap();
        assert_eq!(config.request_type, PackageType::Npm);
    }
}
