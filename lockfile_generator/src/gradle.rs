//! Java gradle ecosystem.

use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::NamedTempFile;

use crate::{Error, FileRelocator, Generator, Result};

pub struct Gradle;

/// Init script passed to gradle to force writing lockfiles.
const INIT_SCRIPT: &[u8] = b"
allprojects {
    dependencyLocking {
        lockAllConfigurations()
    }
}
";

impl Generator for Gradle {
    fn lockfile_path(&self, manifest_path: &Path) -> Result<PathBuf> {
        let project_path = manifest_path
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;
        Ok(project_path.join("gradle.lockfile"))
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        // NOTE: We use a custom command to pass the init script.
        unreachable!()
    }

    fn tool(&self) -> &'static str {
        "Gradle"
    }

    fn check_prerequisites(&self, manifest_path: &Path) -> Result<()> {
        if manifest_path.file_name() == Some(OsStr::new("build.gradle"))
            || manifest_path.file_name() == Some(OsStr::new("build.gradle.kts"))
        {
            Ok(())
        } else {
            Err(Error::InvalidManifest(manifest_path.to_path_buf()))
        }
    }

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

        // Create temporary init script file.
        let mut init_file = NamedTempFile::new()?;
        init_file.write_all(INIT_SCRIPT)?;
        let init_path = init_file.path().to_string_lossy();

        // Generate lockfile at the target location.
        let mut command = Command::new("gradle");
        command.args(["dependencies", "--init-script", &init_path, "--write-locks"]);
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
