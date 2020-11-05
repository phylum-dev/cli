use crate::types::{ApiToken, PackageDescriptor, PackageType};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

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

// TODO: define explicit error types
pub fn save_config(path: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let yaml = serde_yaml::to_string(config)?;
    fs::write(shellexpand::env(path)?.as_ref(), yaml)?;
    Ok(())
}

pub fn parse_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let contents = fs::read_to_string(shellexpand::env(path)?.as_ref())?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Key, UserId};
    use std::str::FromStr;

    #[test]
    fn test_save_config() {
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
        save_config("/tmp/test_config", &config).unwrap();
    }

    #[test]
    fn test_parse_config() {
        let config: Config = parse_config("/tmp/test_config").unwrap();
        assert_eq!(config.request_type, PackageType::Npm);
    }
}
