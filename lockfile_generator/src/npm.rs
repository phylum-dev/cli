//! JavaScript npm ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Npm;

impl Generator for Npm {
    fn lockfile_name(&self) -> &'static str {
        "package-lock.json"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("npm");
        command.args(["install", "--package-lock-only", "--ignore-scripts"]);
        command
    }
}
