use std::env;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// Resolve XDG data directory.
pub fn data_dir() -> Result<PathBuf> {
    if let Some(path) = env_path("XDG_DATA_HOME") {
        return Ok(path);
    }
    Ok(home_dir()?.join(".local/share"))
}

/// Resolve XDG config directory.
pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = env_path("XDG_CONFIG_HOME") {
        return Ok(path);
    }
    Ok(home_dir()?.join(".config"))
}

/// XDG binary directory.
pub fn bin_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".local/bin"))
}

/// User home directory.
pub fn home_dir() -> Result<PathBuf> {
    home::home_dir().ok_or_else(|| anyhow!("Couldn't find the user's home directory"))
}

/// Resolve a path from an environment variable.
pub fn env_path(env_var: &str) -> Option<PathBuf> {
    env::var_os(env_var).filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// Expand leading tildes to the user's home path.
pub fn expand_home_path(path: &str, home: &Path) -> PathBuf {
    path.strip_prefix('~')
        .filter(|path| path.is_empty() || path.starts_with('/'))
        .map(|suffix| home.join(suffix.strip_prefix('/').unwrap_or(suffix)))
        .unwrap_or_else(|| PathBuf::from(path))
}
