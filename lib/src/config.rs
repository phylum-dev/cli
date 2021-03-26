use chrono::{DateTime, Local};
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
    pub user: String,
    pub pass: String,
    pub api_token: Option<ApiToken>,
}

pub type Packages = Vec<PackageDescriptor>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub connection: ConnectionInfo,
    pub auth_info: AuthInfo,
    pub request_type: PackageType,
    pub packages: Option<Packages>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: ProjectId,
    pub name: String,
    pub created_at: DateTime<Local>,
}

// TODO: define explicit error types
pub fn save_config<T>(path: &str, config: &T) -> Result<(), Box<dyn Error>>
where
    T: Serialize,
{
    let yaml = serde_yaml::to_string(config)?;
    fs::write(shellexpand::env(path)?.as_ref(), yaml)?;
    Ok(())
}

pub fn parse_config<T>(path: &str) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    let contents = fs::read_to_string(shellexpand::env(path)?.as_ref())?;
    let config: T = serde_yaml::from_str(&contents)?;
    Ok(config)
}

pub fn read_configuration(path: &str) -> Result<Config, Box<dyn Error>> {
    let mut config: Config = parse_config(path)?;

    // If an api token has been set in the environment, prefer that
    if let Ok(key) = env::var("PHYLUM_API_KEY") {
        log::debug!("Reading api token from environment");
        let token: ApiToken = serde_json::from_str(&key).map_err(|e| {
            log::error!("Malformed PHYLUM_API_KEY: `{}`", key);
            e
        })?;
        config.auth_info.api_token = Some(token);
    }
    Ok(config)
}

pub fn find_project_conf(starting_directory: &str) -> Option<String> {
    let mut path: PathBuf = starting_directory.into();
    let mut attempts = 0;
    const MAX_DEPTH: u8 = 32;

    loop {
        let f = path.join(PROJ_CONF_FILE);
        if f.is_file() {
            break Some(f.to_string_lossy().to_string());
        }

        if attempts > MAX_DEPTH {
            break None;
        }
        path.push("..");
        attempts += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Key, UserId};
    use std::env::temp_dir;
    use std::str::FromStr;

    fn write_test_config() {
        let con = ConnectionInfo {
            uri: "http://127.0.0.1".into(),
        };

        let auth = AuthInfo {
            user: "someone@someorg.com".into(),
            pass: "abcd1234".into(),
            api_token: Some(ApiToken {
                active: true,
                key: Key::from_str("5098fc16-5267-40ed-bf63-338ebdf185fe").unwrap(),
                user_id: UserId::from_str("b4225454-13ee-4019-926e-cd5f8b128e4a").unwrap(),
            }),
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
