//! JavaScript yarn ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Yarn;

impl Generator for Yarn {
    fn lockfile_name(&self) -> &'static str {
        "yarn.lock"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("yarn");
        command.args(["install", "--mode=skip-build", "--mode=update-lockfile"]);
        command
    }
}
