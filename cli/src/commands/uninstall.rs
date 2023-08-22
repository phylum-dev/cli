//! Phylum CLI removal.

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use tempfile::NamedTempFile;

use crate::commands::{CommandResult, ExitCode};
use crate::{dirs, print_user_success, print_user_warning};

/// Handle the `uninstall` subcommand.
pub fn handle_uninstall(matches: &ArgMatches) -> CommandResult {
    if matches.get_flag("purge") {
        purge()?;
    }

    remove_installed_files()?;

    if let Err(err) = Shell::Bash.cleanup() {
        print_user_warning!("BASH config error: {}", err);
    }

    if let Err(err) = Shell::Zsh.cleanup() {
        print_user_warning!("ZSH config error: {}", err);
    }

    Ok(ExitCode::Ok)
}

/// Remove the entire phylum config directory.
fn purge() -> Result<()> {
    let config_dir = dirs::config_dir()?.join("phylum");

    match fs::remove_dir_all(&config_dir) {
        Ok(()) => print_user_success!("Successfully removed {:?}", config_dir),
        Err(err) => print_user_warning!("Could not remove phylum config directory: {}", err),
    }

    Ok(())
}

/// Return Ok for file not found errors. Pass everything else through.
fn ignore_not_found(err: io::Error) -> io::Result<()> {
    if err.kind() == ErrorKind::NotFound {
        Ok(())
    } else {
        Err(err)
    }
}

/// Remove files created by `install.sh`.
fn remove_installed_files() -> Result<()> {
    let data_dir = dirs::data_dir()?.join("phylum");
    let state_dir = dirs::state_dir()?.join("phylum");
    let bin_path = dirs::bin_dir()?.join("phylum");

    let data_result = fs::remove_dir_all(data_dir);
    let state_result = fs::remove_dir_all(state_dir).or_else(ignore_not_found);
    let bin_result = fs::remove_file(bin_path);

    if let Err(err) = &data_result {
        print_user_warning!("Could not remove data directory: {}", err);
    }

    if let Err(err) = &state_result {
        print_user_warning!("Could not remove state directory: {}", err);
    }

    if let Err(err) = &bin_result {
        print_user_warning!("Could not remove phylum executable: {}", err);
    }

    if data_result.is_ok() && state_result.is_ok() && bin_result.is_ok() {
        print_user_success!("Successfully removed installer files");
    }

    Ok(())
}

/// Supported shells.
#[derive(Debug)]
enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    /// Get the shell's config path.
    fn rc_path(&self) -> Result<PathBuf> {
        let home_dir = dirs::home_dir()?;
        let rc_path = match self {
            Self::Bash => home_dir.join(".bashrc"),
            Self::Zsh => home_dir.join(".zshrc"),
        };
        Ok(rc_path)
    }

    /// Get the shell's phylum config path
    fn phylum_path(&self, data_dir: &Path) -> PathBuf {
        let phylum_data = data_dir.join("phylum");
        match self {
            Self::Bash => phylum_data.join("bashrc"),
            Self::Zsh => phylum_data.join("zshrc"),
        }
    }

    /// Remove all lines from the shell config which were added by the
    /// installer.
    fn cleanup(&self) -> Result<()> {
        let rc_path = self.rc_path()?;
        let mut rc_content = fs::read_to_string(&rc_path)?;

        let phylum_path = self.phylum_path(&dirs::data_dir()?);
        let config_line = format!("source {}\n", phylum_path.to_string_lossy());

        // If the installer's config is present, remove it.
        if !rc_content.contains(&config_line) {
            return Ok(());
        }
        rc_content = rc_content.replace(&config_line, "");

        // Write to tempfile on same mountpoint to avoid accidental corruption.
        let rc_dir =
            rc_path.parent().ok_or_else(|| anyhow!("Shell file has no parent directory"))?;
        let mut tmpfile = NamedTempFile::new_in(rc_dir)?;
        tmpfile.write_all(rc_content.as_bytes())?;

        // Swap the tempfile into place.
        tmpfile.persist(rc_path)?;

        print_user_success!("Removed entries from {:?} config.", self);

        Ok(())
    }
}
