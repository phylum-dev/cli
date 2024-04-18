//! Go ecosystem.

use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::{Error, Generator, Result};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Module {
    path: String,
    main: Option<bool>,
    version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GoDep {
    name: String,
    version: String,
}

fn create_go_dep(module: Module) -> Option<GoDep> {
    // Skip main module.
    if module.main.unwrap_or(false) {
        return None;
    }

    let version = module.version?;

    Some(GoDep { name: module.path, version })
}

pub struct Go;

impl Generator for Go {
    fn lockfile_path(&self, _manifest_path: &Path) -> Result<PathBuf> {
        // NOTE: Go's `generate_lockfile` will never write to disk.
        unreachable!()
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("go");
        command.args(["list", "-m", "-json", "all"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Go"
    }

    /// Generate dependencies from `go list` output.
    ///
    /// Since the `go list` never writes any actual lockfile to the disk, we
    /// provide a custom method here which parses this output and transforms it
    /// into a `go.sum` format our lockfile parser expects.
    fn generate_lockfile(&self, manifest_path: &Path) -> Result<String> {
        let canonicalized = fs::canonicalize(manifest_path)?;
        let project_path = canonicalized
            .parent()
            .ok_or_else(|| Error::InvalidManifest(manifest_path.to_path_buf()))?;

        // Execute go list inside the project.
        //
        // We still change directory here since it could impact go's list generation.
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
        // Parse go list STDOUT.
        let stdout = String::from_utf8(output.stdout)?;
        let stream = serde_json::Deserializer::from_str(&stdout).into_iter::<Module>();
        let packages = stream.filter_map(|res| res.ok()).filter_map(create_go_dep);

        let mut lockfile = String::new();
        for pkg in packages {
            let _ = writeln!(lockfile, "{} {} h1:h1", pkg.name, pkg.version);
        }

        Ok(lockfile)
    }

    fn check_prerequisites(&self, manifest_path: &Path) -> Result<()> {
        if manifest_path.file_name() != Some(OsStr::new("go.mod")) {
            Err(Error::InvalidManifest(manifest_path.to_path_buf()))
        } else {
            Ok(())
        }
    }
}
