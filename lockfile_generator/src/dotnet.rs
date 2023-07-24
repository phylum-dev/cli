//! C# .NET ecosystem.

use std::path::Path;
use std::process::Command;

use crate::Generator;

pub struct Dotnet;

impl Generator for Dotnet {
    fn lockfile_name(&self) -> &'static str {
        "packages.lock.json"
    }

    fn command(&self, manifest_path: &Path) -> Command {
        let mut command = Command::new("dotnet");
        command.arg("restore").arg(manifest_path).arg("--use-lock-file");
        command
    }

    fn tool(&self) -> &'static str {
        ".NET"
    }
}
