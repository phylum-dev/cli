//! Sandbox subcommand handling.

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use anyhow::{anyhow, Result};
use birdcage::{Birdcage, Exception, Sandbox};
use clap::ArgMatches;

use crate::commands::extensions::permissions;
use crate::commands::{CommandResult, ExitCode};

/// Entry point for the `sandbox` subcommand.
pub async fn handle_sandbox(matches: &ArgMatches) -> CommandResult {
    // Setup sandbox.
    lock_process(matches)?;

    // Start subprocess.
    let cmd = matches.get_one::<String>("cmd").unwrap();
    let args: Vec<&String> = matches.get_many("args").unwrap_or_default().collect();
    let status = match Command::new(cmd).args(args).status() {
        Ok(status) => status,
        Err(err) => {
            eprintln!("Process {cmd:?} failed to start: {err}");
            return Ok(ExitCode::SandboxStart);
        },
    };

    if let Some(mut code) = status.code() {
        // Remap exit code if it matches our sandbox start failure indicator, to ensure
        // we can detect the failure reliably.
        if code == i32::from(&ExitCode::SandboxStart) {
            code = i32::from(&ExitCode::SandboxStartCollision);
        }

        Ok(ExitCode::Custom(code))
    } else if let Some(signal) = status.signal() {
        Err(anyhow!("Sandbox process failed with signal {signal}"))
    } else {
        unreachable!("Sandbox process terminated without exit code or signal");
    }
}

/// Lock down the current process.
#[cfg(unix)]
fn lock_process(matches: &ArgMatches) -> Result<()> {
    let mut birdcage =
        if matches.get_flag("strict") { Birdcage::new() } else { permissions::default_sandbox()? };

    // Apply filesystem exceptions.
    for path in matches.get_many::<String>("allow-read").unwrap_or_default() {
        permissions::add_exception(&mut birdcage, Exception::Read(path.into()))?;
    }
    for path in matches.get_many::<String>("allow-write").unwrap_or_default() {
        permissions::add_exception(&mut birdcage, Exception::WriteAndRead(path.into()))?;
    }
    for path in matches.get_many::<String>("allow-run").unwrap_or_default() {
        let absolute_path = permissions::resolve_bin_path(path);
        permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead(absolute_path))?;
    }

    // Apply network exceptions.
    if matches.get_flag("allow-net") {
        birdcage.add_exception(Exception::Networking)?;
    }

    // Apply environment variable exceptions.
    for var in matches.get_many::<String>("allow-env").unwrap_or_default() {
        if var == "*" {
            birdcage.add_exception(Exception::FullEnvironment)?;
        } else {
            birdcage.add_exception(Exception::Environment(var.to_owned()))?;
        }
    }

    // Lock down the process.
    birdcage.lock()?;

    Ok(())
}
