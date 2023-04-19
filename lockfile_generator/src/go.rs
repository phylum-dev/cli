//! Go ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Go;

impl Generator for Go {
    fn lockfile_name(&self) -> &'static str {
        "go.sum"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("go");
        command.args(["get", "-d"]);
        command
    }
}
