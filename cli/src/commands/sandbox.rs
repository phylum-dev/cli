//! Sandbox subcommand handling.

use std::fs::File;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Result};
use birdcage::{Birdcage, Exception, Sandbox};
use clap::ArgMatches;

use crate::commands::extensions::permissions;
use crate::commands::{CommandResult, CommandValue, ExitCode};

/// Entry point for the `sandbox` subcommand.
pub async fn handle_sandbox(matches: &ArgMatches) -> CommandResult {
    // Create the IPC error message file before locking the sandbox, so we are still
    // able to access it via this file descriptor.
    let ipc_file = if let Some(path) = matches.get_one::<String>("ipc-path").map(PathBuf::from) {
        Some(File::create(path.join("error"))?)
    } else {
        None
    };

    // Setup sandbox.
    lock_process(matches)?;

    // Start subprocess.
    let cmd = matches.get_one::<String>("cmd").unwrap();
    let args: Vec<&String> = matches.get_many("args").unwrap_or_default().collect();
    let status = Command::new(cmd).args(&args).status().map_err(|e| {
        let error_msg = format!("Sandbox process not started: {cmd}: {e}");

        // If the parent is expecting an error report in `--ipc-path`, provide it.
        if let Some(mut ipc_file) = ipc_file {
            // Filesystem errors will be propagated upwards. There is no non-fallible way of
            // communicating with the parent process in this way.
            if let Err(e) = ipc_file.write_all(error_msg.as_bytes()) {
                return anyhow!(e);
            }
        }
        anyhow!(error_msg)
    })?;

    if let Some(code) = status.code() {
        Ok(CommandValue::Code(ExitCode::Custom(code)))
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
        if matches.get_flag("strict") { Birdcage::new()? } else { permissions::default_sandbox()? };

    // Apply filesystem exceptions.
    for path in matches.get_many::<String>("allow-read").unwrap_or_default() {
        permissions::add_exception(&mut birdcage, Exception::Read(path.into()))?;
    }
    for path in matches.get_many::<String>("allow-write").unwrap_or_default() {
        permissions::add_exception(&mut birdcage, Exception::Write(path.into()))?;
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
