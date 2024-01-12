//! `phylum parse` command for lockfile parsing

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::result::Result as StdResult;
use std::str::FromStr;
use std::{env, fs, io};

use anyhow::{anyhow, Context, Result};
use birdcage::{Birdcage, Exception, Sandbox};
use clap::ArgMatches;
use phylum_lockfile::{LockfileFormat, ParseError, ParsedLockfile};

use crate::commands::extensions::permissions;
use crate::commands::{CommandResult, ExitCode};
use crate::{config, dirs, print_user_failure, print_user_warning};

pub fn lockfile_types(add_auto: bool) -> Vec<&'static str> {
    let mut lockfile_types = LockfileFormat::iter().map(|format| format.name()).collect::<Vec<_>>();

    // Add generic lockfile type.
    if add_auto {
        lockfile_types.push("auto");
    }

    lockfile_types
}

pub fn handle_parse(matches: &ArgMatches) -> CommandResult {
    let sandbox_generation = !matches.get_flag("skip-sandbox");
    let generate_lockfiles = !matches.get_flag("no-generation");

    let project = phylum_project::get_current_project();
    let project_root = project.as_ref().map(|p| p.root());
    let depfiles = config::depfiles(matches, project.as_ref())?;

    let mut pkgs = Vec::new();
    for depfile in depfiles {
        let parse_result = parse_depfile(
            &depfile.path,
            project_root,
            Some(&depfile.depfile_type),
            sandbox_generation,
            generate_lockfiles,
        );

        // Map dedicated exit codes for failures due to disabled generation or
        // unknown dependency file format.
        let parsed_lockfile = match parse_result {
            Ok(parsed_lockfile) => parsed_lockfile,
            Err(err @ ParseError::ManifestWithoutGeneration(_)) => {
                print_user_failure!("Could not parse manifest: {}", err);
                return Ok(ExitCode::ManifestWithoutGeneration);
            },
            Err(err @ ParseError::UnknownManifestFormat(_)) => {
                print_user_failure!("Could not parse manifest: {}", err);
                return Ok(ExitCode::UnknownManifestFormat);
            },
            Err(ParseError::Other(err)) => {
                return Err(err).with_context(|| {
                    format!("could not parse dependency file \"{}\"", depfile.path.display())
                });
            },
        };

        pkgs.append(&mut parsed_lockfile.api_packages());
    }

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok)
}

pub fn handle_parse_sandboxed(matches: &ArgMatches) -> CommandResult {
    let path = PathBuf::from(matches.get_raw("depfile").unwrap().next().unwrap());
    let display_path = matches.get_one::<String>("display-path").unwrap();
    let generate_lockfile = matches.get_flag("generate-lockfile");
    let lockfile_type = matches.get_one::<String>("type");
    let skip_sandbox = matches.get_flag("skip-sandbox");

    if skip_sandbox {
        child_parse_depfile(&path, display_path, lockfile_type, generate_lockfile)
    } else {
        spawn_sandbox(&path, display_path, lockfile_type, generate_lockfile)
    }
}

/// Reexecute `parse-sandboxed` inside the sandbox.
fn spawn_sandbox(
    path: &Path,
    display_path: &str,
    lockfile_type: Option<&String>,
    generate_lockfile: bool,
) -> CommandResult {
    // Setup sandbox for lockfile generation.
    let birdcage = depfile_parsing_sandbox(path)?;

    // Reexecute command inside sandbox.
    let command =
        parse_sandboxed_command(path, display_path, lockfile_type, generate_lockfile, true)?;
    let mut child = birdcage.spawn(command)?;

    // Check for process failure.
    let status = child.wait()?;
    match status.code() {
        Some(code) => Ok(ExitCode::Custom(code)),
        None if !status.success() => Ok(ExitCode::Generic),
        None => Ok(ExitCode::Ok),
    }
}

/// Handle dependency file parsing inside of our sandbox.
fn child_parse_depfile(
    path: &PathBuf,
    display_path: &str,
    lockfile_type: Option<&String>,
    generate_lockfile: bool,
) -> CommandResult {
    let lockfile_type = lockfile_type.map(|t| LockfileFormat::from_str(t).unwrap());

    let generation_path = generate_lockfile.then(|| path.clone());
    let contents = fs::read_to_string(path)?;

    // Parse dependency file.
    let parse_result =
        phylum_lockfile::parse_depfile(&contents, display_path, lockfile_type, generation_path);

    // Map lockfile generation failure to specific exit code.
    let parsed = match parse_result {
        Ok(parsed) => parsed,
        Err(ParseError::ManifestWithoutGeneration(_)) => {
            return Ok(ExitCode::ManifestWithoutGeneration)
        },
        Err(ParseError::UnknownManifestFormat(_)) => return Ok(ExitCode::UnknownManifestFormat),
        Err(ParseError::Other(err)) => return Err(err),
    };

    // Serialize dependency file to stdout.
    println!("{}", serde_json::to_string(&parsed)?);

    Ok(ExitCode::Ok)
}

/// Parse a dependency file.
pub fn parse_depfile(
    path: impl Into<PathBuf>,
    project_root: Option<&PathBuf>,
    depfile_type: Option<&str>,
    sandbox_generation: bool,
    generate_lockfiles: bool,
) -> StdResult<ParsedLockfile, ParseError> {
    // Try and determine dependency file format.
    let path = path.into();
    let (format, path) = match find_depfile_format(&path, depfile_type) {
        Some((format, Some(path))) => (Some(format), path),
        Some((format, None)) => (Some(format), path),
        None => (None, path),
    };

    let display_path = strip_root_path(&path, project_root)?.display().to_string();

    if sandbox_generation && generate_lockfiles {
        // Spawn separate process to allow sandboxing lockfile generation.
        let path = path.canonicalize().map_err(anyhow::Error::from)?;
        let lockfile_type = format.map(|format| format.to_string());
        let mut command = parse_sandboxed_command(
            &path,
            &display_path,
            lockfile_type.as_ref(),
            generate_lockfiles,
            false,
        )?;
        command.stderr(Stdio::inherit());
        let output = command.output().map_err(anyhow::Error::from)?;

        if !output.status.success() {
            // Forward STDOUT to the user on failure.
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("{stdout}");

            // Check exit code to special-case lockfile generation failure.
            if output.status.code() == Some(i32::from(&ExitCode::ManifestWithoutGeneration)) {
                Err(ParseError::ManifestWithoutGeneration(display_path))
            } else if output.status.code() == Some(i32::from(&ExitCode::UnknownManifestFormat)) {
                Err(ParseError::UnknownManifestFormat(display_path))
            } else {
                Err(ParseError::Other(anyhow!("Dependency file parsing failed")))
            }
        } else {
            let json = String::from_utf8_lossy(&output.stdout);
            let parsed_lockfile = serde_json::from_str(&json).map_err(anyhow::Error::from)?;
            Ok(parsed_lockfile)
        }
    } else {
        let contents = fs::read_to_string(&path).map_err(anyhow::Error::from)?;
        let generation_path = generate_lockfiles.then(|| path.clone());

        phylum_lockfile::parse_depfile(&contents, display_path, format, generation_path)
    }
}

/// Find a dependency file's format.
fn find_depfile_format(
    path: &Path,
    depfile_type: Option<&str>,
) -> Option<(LockfileFormat, Option<PathBuf>)> {
    // Determine format from dependency file type.
    if let Some(depfile_type) = depfile_type.filter(|depfile_type| depfile_type != &"auto") {
        let format = LockfileFormat::from_str(depfile_type).unwrap();

        // Skip lockfile analysis when path is only valid manifest.
        let parser = format.parser();
        let lockfile =
            (!parser.is_path_manifest(path) || parser.is_path_lockfile(path)).then(|| path.into());

        return Some((format, lockfile));
    }

    // Determine format based on dependency file path.
    if let Some(format) = phylum_lockfile::get_path_format(path) {
        return Some((format, Some(path.into())));
    }

    // Determine format from manifest path.
    find_manifest_format(path)
}

/// Find a manifest file's format.
fn find_manifest_format(path: &Path) -> Option<(LockfileFormat, Option<PathBuf>)> {
    // Find project root directory.
    let canonicalized = fs::canonicalize(path).ok()?;
    let manifest_dir = canonicalized.parent()?;

    // Find lockfile formats matching this manifest.
    let mut formats =
        LockfileFormat::iter().filter(|format| format.parser().is_path_manifest(path)).peekable();

    // Store first format as fallback.
    let fallback_format = formats.peek().copied();

    // Look for formats which already have a lockfile generated.
    let manifest_lockfile = formats.find_map(|format| {
        let manifest_lockfile = find_direntry_upwards::<32, _>(manifest_dir, |path| {
            format.parser().is_path_lockfile(path)
        })?;
        Some((format, manifest_lockfile))
    });

    // Return existing lockfile or format capable of generating it.
    match manifest_lockfile {
        Some((format, manifest_lockfile)) => {
            print_user_warning!("{path:?} is not a lockfile, using {manifest_lockfile:?} instead");
            Some((format, Some(manifest_lockfile)))
        },
        None => fallback_format.map(|format| (format, None)),
    }
}

/// Find a file by walking from a directory towards the root.
///
/// `MAX_DEPTH` is the maximum number of directory traversals before the search
/// will be abandoned. A `MAX_DEPTH` of `0` will only search the `origin`
/// directory.
fn find_direntry_upwards<const MAX_DEPTH: usize, P>(
    mut origin: &Path,
    mut predicate: P,
) -> Option<PathBuf>
where
    P: FnMut(&Path) -> bool,
{
    for _ in 0..=MAX_DEPTH {
        for entry in fs::read_dir(origin).ok()?.flatten() {
            let path = entry.path();
            if predicate(&path) {
                return Some(path);
            }
        }

        origin = origin.parent()?;
    }

    None
}

/// Modify the provided path to be relative to a given project root or the
/// current directory.
///
/// If a project root is provided and the path starts with this root, the
/// function will return a path relative to this root. If no project root is
/// provided, or if the path doesn't start with the project root, the function
/// will return a path relative to the current directory.
fn strip_root_path(path: &Path, project_root: Option<&PathBuf>) -> Result<PathBuf> {
    let base: Cow<'_, Path> = match project_root {
        Some(p) => p.into(),
        None => env::current_dir()?.into(),
    };

    relative_path(&base, path)
}

/// Computes the relative path from the `from` path to the `to` path.
///
/// This function iterates through the components of both paths in tandem. It
/// skips the common prefix and then, for each remaining component in the
/// starting directory (`from`), it adds a `..` to the result. Finally, it
/// appends the unique components of the target path (`to`).
fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let from = from.canonicalize()?;
    let to = to.canonicalize()?;

    let mut from_components = from.components().peekable();
    let mut to_components = to.components().peekable();

    while from_components.peek() == to_components.peek() {
        from_components.next();
        to_components.next();
    }

    let mut result = PathBuf::new();
    while from_components.next().is_some() {
        result.push("..");
    }
    result.extend(to_components);
    Ok(result)
}

/// Construct command for sandboxed lockfile parsing.
fn parse_sandboxed_command(
    path: &Path,
    display_path: &str,
    lockfile_type: Option<&String>,
    generate_lockfile: bool,
    skip_sandbox: bool,
) -> Result<Command> {
    let current_exe = env::current_exe()?;
    let mut command = Command::new(current_exe);
    command.arg("--no-config");

    command.arg("parse-sandboxed");
    command.arg(path);
    command.arg(display_path);

    if let Some(lockfile_type) = lockfile_type {
        command.args(["--type", &lockfile_type.to_string()]);
    }

    if generate_lockfile {
        command.arg("--generate-lockfile");
    }

    if skip_sandbox {
        command.arg("--skip-sandbox");
    }

    Ok(command)
}

/// Create sandbox for dependency file parsing.
///
/// This sandbox will automatically add all exceptions necessary to generate
/// lockfiles for any ecosystem.
fn depfile_parsing_sandbox(canonical_manifest_path: &Path) -> Result<Birdcage> {
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
    permissions::add_exception(&mut birdcage, Exception::Read("/opt/gradle".into()))?;
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

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use super::*;

    #[test]
    fn find_lockfile_for_manifest() {
        let tempdir = tempfile::tempdir().unwrap();
        let tempdir = tempdir.path().canonicalize().unwrap();

        // Create manifest.
        let manifest_dir = tempdir.join("manifest");
        let manifest_path = manifest_dir.join("Cargo.toml");
        fs::create_dir_all(&manifest_dir).unwrap();
        File::create(&manifest_path).unwrap();

        // Ensure lockfiles below manifest are ignored.

        // Create lockfile below manifest.
        let child_lockfile_dir = manifest_dir.join("sub");
        fs::create_dir_all(&child_lockfile_dir).unwrap();
        File::create(child_lockfile_dir.join("Cargo.lock")).unwrap();

        let (_, path) = find_manifest_format(&manifest_path).unwrap();

        assert_eq!(path, None);

        // Accept lockfiles above the manifest.

        // Create lockfile above manifest.
        let parent_lockfile_path = tempdir.join("Cargo.lock");
        File::create(&parent_lockfile_path).unwrap();

        let (_, path) = find_manifest_format(&manifest_path).unwrap();

        assert_eq!(path, Some(parent_lockfile_path));

        // Prefer lockfiles "closer" to the manifest.

        // Create lockfile in the manifest directory.
        let sibling_lockfile_path = manifest_dir.join("Cargo.lock");
        File::create(&sibling_lockfile_path).unwrap();

        let (_, path) = find_manifest_format(&manifest_path).unwrap();

        assert_eq!(path, Some(sibling_lockfile_path));
    }

    #[test]
    fn test_relative_path() {
        // Set up a temporary directory for testing.
        let tempdir = tempfile::tempdir().unwrap();
        let tempdir = tempdir.path().canonicalize().unwrap();

        // Create a sample project directory named "sample" inside the "projects"
        // directory. Also create a "Cargo.lock" file inside the "sample"
        // directory.
        let sample_dir = tempdir.join("sample");
        let lockfile_path = sample_dir.join("Cargo.lock");
        fs::create_dir_all(&sample_dir).unwrap();
        File::create(&lockfile_path).unwrap();

        // Change the current directory to the "sample" project directory.
        let path = relative_path(&sample_dir, &lockfile_path).unwrap();
        // The path to the lockfile should now be just the filename since it's in the
        // current directory.
        assert_eq!(path.as_os_str(), "Cargo.lock");

        // Create a subdirectory named "sub" within the "sample" project directory.
        let sub_dir = sample_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();

        // Change the current directory to the new "sub" directory.
        let rel_lockfile_path = sub_dir.join("../Cargo.lock");

        // Get the relative path from the sub directory to the lockfile in the sample
        // directory.
        let path = relative_path(&sample_dir, &rel_lockfile_path).unwrap();
        // The path to the lockfile should be the same as before since we are looking
        // relative to the sample directory.
        assert_eq!(path.as_os_str(), "Cargo.lock");

        // Create another "Cargo.lock" file one level above the "sample" directory.
        let above_lockfile_path = tempdir.join("Cargo.lock");
        File::create(above_lockfile_path).unwrap();
        let rel_lockfile_path = sub_dir.join("../../Cargo.lock");

        // Although the current directory is still "sub", get the relative path to the
        // lockfile above the "sample" directory.
        let path = relative_path(&sample_dir, &rel_lockfile_path).unwrap();
        // The path to the lockfile should be relative to the "sample" directory.
        assert_eq!(path.as_os_str(), "../Cargo.lock");
    }
}
