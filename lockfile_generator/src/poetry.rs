//! Python poetry ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Poetry;

impl Generator for Poetry {
    fn lockfile_name(&self) -> &'static str {
        "poetry.lock"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("poetry");
        command.args(["lock"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Poetry"
    }
}
