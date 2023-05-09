//! Python poetry ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Poetry;

impl Generator for Poetry {
    fn lockfile_name(&self) -> &'static str {
        "poetry.lock"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("poetry");
        command.args(["lock"]);
        command
    }
}
