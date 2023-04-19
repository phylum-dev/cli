//! Java gradle ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Gradle;

impl Generator for Gradle {
    fn lockfile_name(&self) -> &'static str {
        "gradle.lockfile"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("gradle");
        command.args(["dependencies", "--write-locks"]);
        command
    }
}
