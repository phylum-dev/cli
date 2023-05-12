//! Rust cargo ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Cargo;

impl Generator for Cargo {
    fn lockfile_name(&self) -> &'static str {
        "Cargo.lock"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("cargo");
        command.args(["generate-lockfile"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Cargo"
    }
}
