//! Python pipenv ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Pip;

impl Generator for Pip {
    fn lockfile_name(&self) -> &'static str {
        "Pipfile.lock"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("pipenv");
        command.args(["lock"]);
        command
    }
}
