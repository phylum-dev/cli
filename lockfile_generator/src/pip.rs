//! Python pipenv ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Pip;

impl Generator for Pip {
    fn lockfile_name(&self) -> &'static str {
        "Pipfile.lock"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("pipenv");
        command.args(["lock"]);
        command
    }
}
