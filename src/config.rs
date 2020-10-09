use std::fs;
use std::error::Error;
use yaml_rust::{Yaml, YamlLoader};
use crate::types::PackageDescriptor;

#[derive(Debug)]
pub struct ConnectionInfo {
    pub uri: String,
    pub user: String,
    pub pass: String,
}

pub type Packages = Vec<PackageDescriptor>;

pub struct Config {
    pub connection: ConnectionInfo,
    pub packages: Option<Packages>,
}

// TODO: define explicit error types

fn parse_package(p: &Yaml) -> Result<PackageDescriptor, Box<dyn Error>> {
    let name = p["name"].as_str().ok_or("Couldn't read package name")?;
    let version = p["version"].as_str().ok_or("Couldn't read package version")?;
    let pkg_type = p["type"].as_str().ok_or("Couldn't read package type")?;
    let pkg_type = serde_json::from_str(&format!("\"{}\"", pkg_type))?;

    Ok(PackageDescriptor { name: name.to_string(), version: version.to_string(), pkg_type })
}

pub fn parse_config(config: &str) -> Result<Config, Box<dyn Error>> {
    let config = fs::read_to_string(config)?;
    let settings = YamlLoader::load_from_str(&config)?;
    let connection_info = &settings[0]["connection"][0];
    let s = |s: &str| s.to_string();
    let uri = connection_info["url"].as_str().map(s).ok_or("Couldn't read connection url")?;
    let user = connection_info["login"].as_str().map(s).ok_or("Couldn't read login")?;
    let pass = connection_info["password"].as_str().map(s).ok_or("Couldn't read password")?;

    let package_entries = settings[0]["packages"].as_vec();

    if package_entries.is_some() {
        let packages: Result<Vec<_>, _>  = package_entries.
            unwrap().
            iter().
            map(parse_package).
            collect();

        Ok(Config {
            connection: ConnectionInfo { uri, user, pass },
            packages: Some(packages?)
        })
    } else {
        Ok(Config { 
            connection: ConnectionInfo { uri, user, pass },
            packages: None,
        })
    }
}