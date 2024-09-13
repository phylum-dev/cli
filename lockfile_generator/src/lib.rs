use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf, StripPrefixError};
use std::process::{Command, Output, Stdio};
use std::string::FromUtf8Error;
use std::{fs, io};

use serde_json::Error as JsonError;

pub mod bundler;
pub mod cargo;
pub mod dotnet;
pub mod go;
pub mod gradle;
pub mod maven;
pub mod npm;
pub mod pip;
pub mod pipenv;
pub mod pnpm;
pub mod poetry;
pub mod yarn;

/// Lockfile generation.
pub trait Generator {
    /// Lockfile path.
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf>;

    /// Command for generating the lockfile.
    fn command(&self, manifest_path: &Path) -> Command;

    /// Name of the tool used to generate the lockfile
    fn tool(&self) -> &'static str;

    /// List of files conflicting with lockfile generation.
    ///
    /// These files are temporarily renamed during lockfile generation to ensure
    /// the correct lockfile is updated.
    fn conflicting_files(&self, manifest_path: &Path) -> Result<Vec<PathBuf>> {
        Ok(vec![self.lockfile_path(manifest_path)?])
    }

    /// Verify that all the prerequisites for lockfile generation are met.
    fn check_prerequisites(&self, _manifest_path: &Path) -> Result<()> {
        Ok(())
    }

    /// Generate the lockfile for a project.
    ///
    /// This will ignore all existing lockfiles and create a new lockfile based
    /// on the current project configuration.
    fn generate_lockfile(&self, manifest_path: &Path) -> Result<String> {
        self.check_prerequisites(manifest_path)?;

        let canonicalized = dunce::canonicalize(manifest_path)?;
        let project_path = canonicalized
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        // Move files which interfere with lockfile generation.
        let _relocators = self
            .conflicting_files(&canonicalized)?
            .drain(..)
            .map(FileRelocator::new)
            .collect::<Result<Vec<_>>>()?;

        // Generate lockfile at the target location.
        let mut command = self.command(&canonicalized);
        command.current_dir(project_path);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());

        // Provide better error message, including the failed program's name.
        let output = command.output().map_err(|err| {
            let program = format!("{:?}", command.get_program());
            Error::ProcessCreation(program, self.tool().to_string(), err)
        })?;

        // Ensure generation was successful.
        if !output.status.success() {
            return Err(Error::NonZeroExit(output));
        }

        // Ensure lockfile was created.
        let lockfile_path = self.lockfile_path(&canonicalized)?;
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
#[derive(thiserror::Error, Debug)]
pub enum Error {
    Anyhow(#[from] anyhow::Error),
    InvalidManifest(PathBuf),
    InvalidUtf8(#[from] FromUtf8Error),
    Io(#[from] io::Error),
    Json(#[from] JsonError),
    NonZeroExit(Output),
    PipReportVersionMismatch(&'static str, String),
    ProcessCreation(String, String, #[source] io::Error),
    StripPrefix(#[from] StripPrefixError),
    UnsupportedCommandVersion(&'static str, &'static str, String),
    NoLockfileGenerated,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anyhow(err) => write!(f, "{err}"),
            Self::InvalidManifest(path) => write!(f, "invalid manifest path: {path:?}"),
            Self::InvalidUtf8(_) => write!(f, "utf8 parsing error"),
            Self::Io(_) => write!(f, "I/O error"),
            Self::Json(_) => write!(f, "json parsing error"),
            Self::NonZeroExit(output) => {
                write!(f, "package manager quit unexpectedly (code: {:?}", output.status.code())?;
                #[cfg(unix)]
                write!(f, ", signal: {:?}", output.status.signal())?;
                write!(f, ")")?;

                if !output.stderr.is_empty() {
                    write!(f, "\n    STDERR:")?;
                    for line in String::from_utf8_lossy(&output.stderr).lines() {
                        write!(f, "\n        {line}")?;
                    }
                }

                if !output.stdout.is_empty() {
                    write!(f, "\n    STDOUT:")?;
                    for line in String::from_utf8_lossy(&output.stdout).lines() {
                        write!(f, "\n        {line}")?;
                    }
                }

                Ok(())
            },
            Self::PipReportVersionMismatch(expected, version) => {
                write!(f, "unsupported pip report version {version:?}, expected {expected:?}")
            },
            Self::ProcessCreation(command, tool_name, _) => {
                write!(f, "failed to spawn command {command}: Is {tool_name} installed?")
            },
            Self::StripPrefix(_) => write!(f, "could not strip path prefix"),
            Self::UnsupportedCommandVersion(command, expected_version, version) => write!(
                f,
                "unsupported {command:?} version {version:?}, expected {expected_version:?}"
            ),
            Self::NoLockfileGenerated => write!(f, "no lockfile was generated"),
        }
    }
}
