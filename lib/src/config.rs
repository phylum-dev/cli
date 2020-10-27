use crate::types::{PackageDescriptor, PackageType};
use std::error::Error;
use std::fs;
use std::str::FromStr;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug)]
pub struct ConnectionInfo {
    pub uri: String,
    pub user: String,
    pub pass: String,
}

pub type Packages = Vec<PackageDescriptor>;

pub struct Config {
    pub connection: ConnectionInfo,
    pub request_type: PackageType,
    pub packages: Option<Packages>,
}

// TODO: define explicit error types

fn parse_package(p: &Yaml, ptype: &PackageType) -> Result<PackageDescriptor, Box<dyn Error>> {
    let name = p["name"].as_str().ok_or("Couldn't read package name")?;
    let version = p["version"]
        .as_str()
        .ok_or("Couldn't read package version")?;

    Ok(PackageDescriptor {
        name: name.to_string(),
        version: version.to_string(),
        r#type: ptype.to_owned(),
    })
}

pub fn parse_config(config: &str) -> Result<Config, Box<dyn Error>> {
    let config = fs::read_to_string(config)?;
    let settings = YamlLoader::load_from_str(&config)?;
    let connection_info = &settings[0]["connection"][0];
    let s = |s: &str| s.to_string();
    let uri = connection_info["url"]
        .as_str()
        .map(s)
        .ok_or("Couldn't read connection url")?;
    let user = connection_info["login"]
        .as_str()
        .map(s)
        .ok_or("Couldn't read login")?;
    let pass = connection_info["password"]
        .as_str()
        .map(s)
        .ok_or("Couldn't read password")?;

    let request_type = &settings[0]["request_type"][0].as_str().unwrap_or("npm");
    //let request_type = serde_json::from_str(&format!("\"{}\"", request_type))?;
    let request_type = PackageType::from_str(request_type).unwrap_or(PackageType::Npm);

    let package_entries = settings[0]["packages"].as_vec();

    if let Some(package_entries) = package_entries {
        let packages: Result<Vec<_>, _> = package_entries
            .iter()
            .map(|p| parse_package(p, &request_type))
            .collect();

        Ok(Config {
            connection: ConnectionInfo { uri, user, pass },
            request_type,
            packages: Some(packages?),
        })
    } else {
        Ok(Config {
            connection: ConnectionInfo { uri, user, pass },
            request_type,
            packages: None,
        })
    }
}
