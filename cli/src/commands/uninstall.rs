//! Phylum CLI removal.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use tempfile::NamedTempFile;

use crate::commands::{CommandResult, ExitCode};
use crate::{config, print_user_success, print_user_warning};

/// Handle the `uninstall` subcommand.
pub fn handle_uninstall(matches: &ArgMatches) -> CommandResult {
    let home_dir = home_dir()?;

    if matches.is_present("purge") {
        purge(&home_dir);
    }

    remove_installed_files(&home_dir);

    if let Err(err) = Shell::Bash.cleanup(&home_dir) {
        print_user_warning!("BASH config error: {}", err);
    }

    if let Err(err) = Shell::Zsh.cleanup(&home_dir) {
        print_user_warning!("ZSH config error: {}", err);
    }

    Ok(ExitCode::Ok.into())
}

/// Remove the entire `~/.config/phylum` directory.
fn purge(home_dir: &Path) {
    let config_dir = config::config_dir(home_dir).join("phylum");

    match fs::remove_dir_all(&config_dir) {
        Ok(()) => print_user_success!("Successfully removed {:?}", config_dir),
        Err(err) => {
            print_user_warning!("Could not remove phylum config directory: {}", err);
        }
    }
}

/// Remove files created by `install.sh`.
fn remove_installed_files(home_dir: &Path) {
    let data_dir = data_dir(home_dir).join("phylum");
    let bin_path = bin_dir(home_dir).join("phylum");

    let data_result = fs::remove_dir_all(data_dir);
    let bin_result = fs::remove_file(bin_path);

    if let Err(err) = &data_result {
        print_user_warning!("Could not remove data directory: {}", err);
    }

    if let Err(err) = &bin_result {
        print_user_warning!("Could not remove phylum executable: {}", err);
    }

    if data_result.is_ok() && bin_result.is_ok() {
        print_user_success!("Successfully removed installer files");
    }
}

/// XDG data directory.
fn data_dir(home_dir: &Path) -> PathBuf {
    env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".local/share"))
}

/// XDG binary directory.
fn bin_dir(home_dir: &Path) -> PathBuf {
    home_dir.join(".local/bin")
}

/// Supported shells.
#[derive(Debug)]
enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    /// Get the shell's config path.
    fn rc_path(&self, home_dir: &Path) -> PathBuf {
        match self {
            Self::Bash => home_dir.join(".bashrc"),
            Self::Zsh => home_dir.join(".zshrc"),
        }
    }

    /// Get the shell's phylum config path
    fn phylum_path(&self, home_dir: &Path) -> PathBuf {
        let data_dir = data_dir(home_dir).join("phylum");
        match self {
            Self::Bash => data_dir.join("bashrc"),
            Self::Zsh => data_dir.join("zshrc"),
        }
    }

    /// Remove all lines from the shell config which were added by the installer.
    fn cleanup(&self, home_dir: &Path) -> Result<()> {
        let rc_path = self.rc_path(home_dir);
        let mut rc_content = fs::read_to_string(&rc_path)?;

        let phylum_path = self.phylum_path(home_dir);
        let config_line = format!("source {}\n", phylum_path.to_string_lossy());

        // If the installer's config is present, remove it.
        if !rc_content.contains(&config_line) {
            return Ok(());
        }
        rc_content = rc_content.replace(&config_line, "");

        // Write to tempfile on same mountpoint to avoid accidental corruption.
        let rc_dir = rc_path
            .parent()
            .ok_or_else(|| anyhow!("Shell file has no parent directory"))?;
        let mut tmpfile = NamedTempFile::new_in(rc_dir)?;
        tmpfile.write_all(rc_content.as_bytes())?;

        // Swap the tempfile into place.
        tmpfile.persist(rc_path)?;

        print_user_success!("Removed entries from {:?} config.", self);

        Ok(())
    }
}

/// Get the user's home directory.
fn home_dir() -> Result<PathBuf> {
    home::home_dir().ok_or(anyhow!("Unable to find home directory"))
}
