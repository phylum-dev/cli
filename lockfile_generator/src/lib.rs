use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

pub mod cargo;
pub mod go;
pub mod gradle;
pub mod maven;
pub mod npm;
pub mod pip;
pub mod poetry;
pub mod python_requirements;
pub mod yarn;

/// Lockfile generation.
pub trait Generator {
    /// Lockfile file name.
    fn lockfile_name(&self) -> &'static str;

    /// Command for generating the lockfile.
    fn command(&self) -> Command;

    /// Generate the lockfile for a project.
    ///
    /// This will ignore all existing lockfiles and create a new lockfile based
    /// on the current project configuration.
    fn generate_lockfile(&self, project_path: &Path) -> Result<String> {
        // Move existing lockfile so we do not overwrite it.
        let lockfile_path = project_path.join(self.lockfile_name());
        let _relocator = if lockfile_path.exists() {
            Some(FileRelocator::new(lockfile_path.clone())?)
        } else {
            None
        };

        // Generate lockfile at the target location.
        let mut command = self.command();
        command.current_dir(project_path);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        // Provide better error message, including the failed program's name.
        let mut child = command.spawn().map_err(|err| {
            let program = command.get_program().to_string_lossy().into_owned();
            Error::ProcessCreation(program, err)
        })?;

        // Ensure generation was successful.
        let status = child.wait()?;
        if !status.success() {
            return Err(Error::NonZeroExit);
        }

        // Ensure lockfile was created.
        if !lockfile_path.exists() {
            return Err(Error::NoLockfileGenerated);
        }

        // Read lockfile contents.
        let lockfile = fs::read_to_string(&lockfile_path)?;

        // Cleanup lockfile.
        fs::remove_file(lockfile_path)?;

        Ok(lockfile)
    }
}

/// Temporarily move a file to a different location.
///
/// This utility moves a file to a backup location in the same directory and
/// automatically restores it to its original location on drop.
struct FileRelocator {
    original_path: PathBuf,
    backup_path: OsString,
}

impl Drop for FileRelocator {
    fn drop(&mut self) {
        // We can't do anything about failure here, but the original file should stay
        // around allowing users to still resolve these issues manually.
        let _ = fs::rename(&self.backup_path, &self.original_path);
    }
}

impl FileRelocator {
    fn new(path: PathBuf) -> Result<Self> {
        // Relocate the file.
        let mut backup_path = path.clone().into_os_string();
        backup_path.push(".phylum_bak");
        fs::rename(&path, &backup_path)?;

        Ok(Self { original_path: path, backup_path })
    }
}

/// Lockfile generation result.
pub type Result<T> = std::result::Result<T, Error>;

/// Lockfile generation error.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to run command {0:?}: {1}")]
    ProcessCreation(String, io::Error),
    #[error("package manager exited with non-zero status")]
    NonZeroExit,
    #[error("no lockfile was generated")]
    NoLockfileGenerated,
}
