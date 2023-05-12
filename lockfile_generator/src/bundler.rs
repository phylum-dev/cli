//! Ruby bundler ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Bundler;

impl Generator for Bundler {
    fn lockfile_name(&self) -> &'static str {
        "Gemfile.lock"
    }

    fn command(&self, _manifest_path: &Path) -> Command {
        let mut command = Command::new("bundle");
        command.args(["lock"]);
        command
    }

    fn tool(&self) -> &'static str {
        "Bundler"
    }
}
