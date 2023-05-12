//! Python pip ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct PipTools;

impl Generator for PipTools {
    fn lockfile_name(&self) -> &'static str {
        "requirements-locked.txt"
    }

    fn command(&self, manifest_path: &Path) -> Command {
        let mut command = Command::new("pip-compile");
        command.arg("-o").arg(self.lockfile_name()).arg(manifest_path);
        command
    }
}
