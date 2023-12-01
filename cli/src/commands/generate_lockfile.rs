//! `phylum generate-lockfile` subcommand.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use birdcage::{Birdcage, Exception, Sandbox};
use clap::ArgMatches;
use phylum_lockfile::LockfileFormat;

use crate::commands::extensions::permissions;
use crate::commands::{CommandResult, ExitCode};
use crate::dirs;

/// Handle `phylum generate-lockfile` subcommand.
pub fn handle_command(matches: &ArgMatches) -> CommandResult {
    let lockfile_type = matches.get_one::<String>("lockfile-type").unwrap();
    let manifest = matches.get_raw("manifest").unwrap().next().unwrap();
    let skip_sandbox = matches.get_flag("skip-sandbox");

    if skip_sandbox {
        generate_lockfile(lockfile_type, manifest)
    } else {
        spawn_sandbox(lockfile_type, manifest)
    }
}

/// Reexecute command inside the sandbox.
fn spawn_sandbox(lockfile_type: &String, manifest: &OsStr) -> CommandResult {
    let manifest_path = PathBuf::from(manifest);

    // Setup sandbox for lockfile generation.
    let birdcage = lockfile_generation_sandbox(&manifest_path)?;

    // Reexecute command inside sandbox.
    let current_exe = env::current_exe()?;
    let mut command = Command::new(current_exe);
    command.arg("generate-lockfile");
    command.arg("--skip-sandbox");
    command.arg(lockfile_type);
    command.arg(manifest);
    let mut child = birdcage.spawn(command)?;

    // Check for process failure.
    let status = child.wait()?;
    match status.code() {
        Some(code) => Ok(ExitCode::Custom(code)),
        None if !status.success() => Ok(ExitCode::Generic),
        None => Ok(ExitCode::Ok),
    }
}

/// Generate lockfile and write it to STDOUT.
fn generate_lockfile(lockfile_type: &String, manifest: &OsStr) -> CommandResult {
    let manifest_path = PathBuf::from(manifest);

    // Get generator for the lockfile type.
    let lockfile_format = lockfile_type.parse::<LockfileFormat>().unwrap();
    let generator = lockfile_format.parser().generator().unwrap();

    // Generate the lockfile.
    let generated_lockfile = generator
        .generate_lockfile(&manifest_path)
        .context("lockfile generation subcommand failed")?;

    // Write lockfile to stdout.
    println!("{}", generated_lockfile);

    Ok(ExitCode::Ok)
}

/// Create sandbox with exceptions allowing generation of any lockfile.
fn lockfile_generation_sandbox(canonical_manifest_path: &Path) -> Result<Birdcage> {
    let mut birdcage = permissions::default_sandbox()?;

    // Allow all networking.
    birdcage.add_exception(Exception::Networking)?;

    // Allow reexecuting phylum.
    let current_exe = env::current_exe()?;
    permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead(current_exe))?;

    // Add exception for the manifest's parent directory.
    let project_path = canonical_manifest_path.parent().expect("Invalid manifest path");
    permissions::add_exception(&mut birdcage, Exception::WriteAndRead(project_path.into()))?;

    // Add exception for all the executables required for generation.
    let ecosystem_bins = [
        "cargo", "bundle", "mvn", "gradle", "npm", "pnpm", "yarn", "python3", "pipenv", "poetry",
        "go", "dotnet",
    ];
    for bin in ecosystem_bins {
        let absolute_path = permissions::resolve_bin_path(bin);
        permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead(absolute_path))?;
    }

    // Allow any executable in common binary directories.
    //
    // Reading binaries shouldn't be an attack vector, but significantly simplifies
    // complex ecosystems (like Python's symlinks).
    permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/bin".into()))?;
    permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead("/bin".into()))?;

    // Add paths required by specific ecosystems.
    let home = dirs::home_dir()?;
    // Cargo.
    permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead(home.join(".rustup")))?;
    permissions::add_exception(&mut birdcage, Exception::ExecuteAndRead(home.join(".cargo")))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/etc/passwd".into()))?;
    // Bundle.
    permissions::add_exception(&mut birdcage, Exception::Read("/dev/urandom".into()))?;
    // Maven.
    permissions::add_exception(&mut birdcage, Exception::WriteAndRead(home.join(".m2")))?;
    permissions::add_exception(&mut birdcage, Exception::WriteAndRead("/var/folders".into()))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/opt/maven".into()))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/etc/java-openjdk".into()))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/usr/local/Cellar/maven".into()))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/usr/local/Cellar/openjdk".into()))?;
    permissions::add_exception(
        &mut birdcage,
        Exception::Read("/opt/homebrew/Cellar/maven".into()),
    )?;
    permissions::add_exception(
        &mut birdcage,
        Exception::Read("/opt/homebrew/Cellar/openjdk".into()),
    )?;
    // Gradle.
    permissions::add_exception(&mut birdcage, Exception::WriteAndRead(home.join(".gradle")))?;
    permissions::add_exception(
        &mut birdcage,
        Exception::Read("/usr/share/java/gradle/lib".into()),
    )?;
    permissions::add_exception(&mut birdcage, Exception::Read("/usr/local/Cellar/gradle".into()))?;
    permissions::add_exception(
        &mut birdcage,
        Exception::Read("/opt/homebrew/Cellar/gradle".into()),
    )?;
    // Pnpm.
    permissions::add_exception(&mut birdcage, Exception::Read("/tmp".into()))?;
    // Yarn.
    permissions::add_exception(&mut birdcage, Exception::Read(home.join("./yarn")))?;

    Ok(birdcage)
}
