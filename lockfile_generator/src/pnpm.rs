//! JavaScript pnpm ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Pnpm;

impl Generator for Pnpm {
    fn lockfile_name(&self) -> &'static str {
        "pnpm-lock.yaml"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("pnpm");
        command.args(["install", "--lockfile-only", "--ignore-scripts"]);
        command
    }

    fn tool(&self) -> &'static str {
        "pnpm"
    }
}
