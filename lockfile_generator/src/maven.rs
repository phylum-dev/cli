//! Java maven ecosystem.

use std::process::Command;

use crate::Generator;

pub struct Maven;

impl Generator for Maven {
    fn lockfile_name(&self) -> &'static str {
        "effective-pom.xml"
    }

    fn command(&self) -> Command {
        let mut command = Command::new("mvn");
        command.args(["help:effective-pom", &format!("-Doutput={}", self.lockfile_name())]);
        command
    }
}
