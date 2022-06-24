// TODO: Things to figure out:
//  - Tests all have to be called manually, that sucks!
//  - Tests run after each other, one failure prevents next one from running
//  - No status output about success

use std::path::{Path, PathBuf};
use std::{env, fs};

use phylum_cli::config::{Config, ConnectionInfo};
use tempfile::TempDir;

// TODO: Hardcode this?
const API_URL: &str = "https://api.staging.phylum.io";
const PROJECT_NAME: &str = "integration-tests";

mod extension;

fn main() {}

/// Create config file for the desired environment.
pub fn create_config(dir: &Path) -> PathBuf {
    let config = Config { connection: ConnectionInfo { uri: API_URL.into() }, ..Config::default() };

    let config_path = dir.join("settings.yml");
    let config_yaml = serde_yaml::to_string(&config).expect("serialize config");
    fs::write(&config_path, config_yaml.as_bytes()).expect("writing config");

    config_path
}

/// Create a simple test lockfile.
pub fn create_lockfile(dir: &Path) -> PathBuf {
    let lockfile = dir.join("yarn.lock");
    fs::write(
        &lockfile,
        br#"
        __metadata:
          version: 6
          cacheKey: 8
        "accepts@npm:~1.3.8":
          version: 1.3.8
          resolution: "accepts@npm:1.3.8"
          checksum: 50c43d32e7b50285ebe84b613ee4a3aa426715a7d131b65b786e2ead0fd76b6b60091b9916d3478a75f11f162628a2139991b6c03ab3f1d9ab7c86075dc8eab4
          languageName: node
          linkType: hard
    "#,
    )
    .unwrap();
    lockfile
}

/// Ensure the specified project exists.
pub fn create_project() -> &'static str {
    // TODO

    PROJECT_NAME
}
