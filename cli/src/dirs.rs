use std::env;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// Resolve XDG data directory.
pub fn data_dir() -> Result<PathBuf> {
    xdg_dir("XDG_DATA_HOME", ".local/share")
}

/// Resolve XDG config directory.
pub fn config_dir() -> Result<PathBuf> {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// XDG binary directory.
pub fn bin_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".local/bin"))
}

/// User home directory.
pub fn home_dir() -> Result<PathBuf> {
    home::home_dir().ok_or_else(|| anyhow!("Couldn't find the user's home directory"))
}

/// Resolve an XDG directory.
pub fn xdg_dir(env_var: &str, path_suffix: impl AsRef<Path>) -> Result<PathBuf> {
    env::var_os(env_var)
        .filter(|s| !s.is_empty())
        .map(|var| Ok(PathBuf::from(var)))
        .unwrap_or_else(|| Ok(home_dir()?.join(path_suffix)))
}

/// Expand leading tildes to the user's home path.
pub fn expand_home_path(path: &str, home: &Path) -> PathBuf {
    path.strip_prefix('~')
        .filter(|path| path.is_empty() || path.starts_with('/'))
        .map(|suffix| home.join(suffix.strip_prefix('/').unwrap_or(suffix)))
        .unwrap_or_else(|| PathBuf::from(path))
}
