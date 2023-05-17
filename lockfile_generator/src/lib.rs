use std::error::Error as StdError;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

pub mod bundler;
pub mod cargo;
pub mod go;
pub mod gradle;
pub mod maven;
pub mod npm;
pub mod pip_tools;
pub mod pipenv;
pub mod poetry;
pub mod yarn;

/// Lockfile generation.
pub trait Generator {
    /// Lockfile file name.
    fn lockfile_name(&self) -> &'static str;

    /// Command for generating the lockfile.
    fn command(&self, manifest_path: &Path) -> Command;

    /// List of files conflicting with lockfile generation.
    ///
    /// These files are temporarily renamed during lockfile generation to ensure
    /// the correct lockfile is updated.
    fn conflicting_files(&self) -> Vec<&'static str> {
        vec![self.lockfile_name()]
    }

    /// Generate the lockfile for a project.
    ///
    /// This will ignore all existing lockfiles and create a new lockfile based
    /// on the current project configuration.
    fn generate_lockfile(&self, manifest_path: &Path) -> Result<String> {
        let canonicalized = fs::canonicalize(manifest_path)?;
        let project_path = canonicalized
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        // Move files which interfere with lockfile generation.
        let _relocators = self
            .conflicting_files()
            .drain(..)
            .map(|file| FileRelocator::new(project_path.join(file)))
            .collect::<Result<Vec<_>>>()?;

        // Generate lockfile at the target location.
        let mut command = self.command(&canonicalized);
        command.current_dir(project_path);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());

        // Provide better error message, including the failed program's name.
        let output = command.output().map_err(|err| {
            let program = format!("{:?}", command.get_program());
            Error::ProcessCreation(program, err)
        })?;

        // Ensure generation was successful.
        if !output.status.success() {
            let code = output.status.code();
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::NonZeroExit(code, stderr.into()));
        }

        // Ensure lockfile was created.
        let lockfile_path = project_path.join(self.lockfile_name());
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
    fn new(path: PathBuf) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        // Relocate the file.
        let mut backup_path = path.clone().into_os_string();
        backup_path.push(".phylum_bak");
        fs::rename(&path, &backup_path)?;

        Ok(Some(Self { original_path: path, backup_path }))
    }
}

/// Lockfile generation result.
pub type Result<T> = std::result::Result<T, Error>;

/// Lockfile generation error.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidManifest(PathBuf),
    ProcessCreation(String, io::Error),
    NonZeroExit(Option<i32>, String),
    NoLockfileGenerated,
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::ProcessCreation(_, err) => Some(err),
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => write!(f, "I/O error"),
            Self::InvalidManifest(path) => write!(f, "invalid manifest path: {path:?}"),
            Self::ProcessCreation(program, _) => {
                write!(f, "failed to spawn command {program}")
            },
            Self::NonZeroExit(Some(code), stderr) => {
                write!(f, "package manager exited with error code {code}:\n\n{stderr}")
            },
            Self::NonZeroExit(None, stderr) => {
                write!(f, "package manager quit unexpectedly:\n\n{stderr}")
            },
            Self::NoLockfileGenerated => write!(f, "no lockfile was generated"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
