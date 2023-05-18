//! JavaScript npm ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Npm;

impl Generator for Npm {
    fn lockfile_name(&self) -> &'static str {
        "package-lock.json"
    }

    fn conflicting_files(&self) -> Vec<&'static str> {
        vec![self.lockfile_name(), "npm-shrinkwrap.json", "yarn.lock"]
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("npm");
        command.args(["install", "--package-lock-only", "--ignore-scripts"]);
        command
    }

    fn tool(&self) -> &'static str {
        "npm"
    }
}
