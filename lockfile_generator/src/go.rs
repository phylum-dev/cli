//! Go ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Go;

impl Generator for Go {
    fn lockfile_name(&self) -> &'static str {
        "go.sum"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("go");
        command.args(["get", "-d"]);
        command
    }
}
