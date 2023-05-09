//! Java maven ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Maven;

impl Generator for Maven {
    fn lockfile_name(&self) -> &'static str {
        "effective-pom.xml"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("mvn");
        command.args(["help:effective-pom", &format!("-Doutput={}", self.lockfile_name())]);
        command
    }
}
