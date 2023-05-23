//! Python pip ecosystem.

use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::{Error, Generator, Result};

pub struct Pip;

impl Generator for Pip {
    fn lockfile_name(&self) -> &'static str {
        // NOTE: Pip's `generate_lockfile` will never write to disk.
        unreachable!()
    }

    fn command(&self, manifest_path: &Path) -> Command {
        let mut command = Command::new("pip3");
        command.args(["install", "--quiet", "--report", "-", "--dry-run"]);

        // Check if we got a requirements file or a setup.py/pyproject.toml.
        let is_requirements_file =
            manifest_path.file_name().and_then(|f| f.to_str()).map_or(false, |file_name| {
                file_name == "requirements.in"
                    || (file_name.starts_with("requirements") && file_name.ends_with(".txt"))
            });

        if is_requirements_file {
            command.arg("-r").arg(manifest_path);
        } else {
            command.arg(".");
        }

        command
    }

    fn tool(&self) -> &'static str {
        "pip"
    }

    /// Generate virtual requirements.txt from dry-run output.
    ///
    /// Since the `pip --report` never writes any actual lockfile to the disk,
    /// we provide a custom method here which parses this output and transforms
    /// it into the locked requirements.txt format our lockfile parser expects.
    fn generate_lockfile(&self, manifest_path: &Path) -> Result<String> {
        let canonicalized = fs::canonicalize(manifest_path)?;
        let project_path = canonicalized
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        // Execute pip inside the project.
        //
        // We still change directory here since it could impact pip's report generation.
        let mut command = self.command(&canonicalized);
        command.current_dir(project_path);
        command.stdin(Stdio::null());

        // Provide better error message, including the failed program's name.
        let output = command.output().map_err(|err| {
            let program = format!("{:?}", command.get_program());
            Error::ProcessCreation(program, self.tool().to_string(), err)
        })?;

        // Ensure generation was successful.
        if !output.status.success() {
            let code = output.status.code();
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::NonZeroExit(code, stderr.into()));
        }

        // Parse pip install report STDOUT.
        let stdout = String::from_utf8(output.stdout)?;
        let report: Report = serde_json::from_str(&stdout)?;

        // Abort if the report version isn't supported.
        let supported_version = "1";
        if report.version != supported_version {
            return Err(Error::PipReportVersionMismatch(supported_version, report.version));
        }

        // Create the virtual requirements.txt lockfile.
        let mut lockfile = String::new();
        for package in report.install {
            let name = package.metadata.name;
            if package.is_direct {
                let _ = writeln!(lockfile, "{} @ {}", name, package.download_info.url);
            } else {
                let _ = writeln!(lockfile, "{}=={}", name, package.metadata.version);
            }
        }

        Ok(lockfile)
    }
}

/// Pip install report output.
#[derive(Deserialize, Debug)]
struct Report {
    version: String,
    install: Vec<ReportPackage>,
}

/// Pip install report package.
#[derive(Deserialize, Debug)]
struct ReportPackage {
    download_info: PackageDownloadInfo,
    metadata: PackageMetadata,
    is_direct: bool,
}

/// Pip install report package metadata.
#[derive(Deserialize, Debug)]
struct PackageMetadata {
    name: String,
    version: String,
}

/// Partial pip install report download info.
#[derive(Deserialize, Debug)]
struct PackageDownloadInfo {
    url: String,
}
