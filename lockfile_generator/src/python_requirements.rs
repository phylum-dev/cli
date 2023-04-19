//! Python pip ecosystem.

use std::process::Command;

use crate::Generator;

pub struct PythonRequirements;

impl Generator for PythonRequirements {
    fn lockfile_name(&self) -> &'static str {
        "requirements-locked.txt"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("pip-compile");
        command.args(["-o", self.lockfile_name()]);
        command
    }
}
