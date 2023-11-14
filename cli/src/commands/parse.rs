//! `phylum parse` command for lockfile parsing

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use anyhow::{anyhow, Context, Result};
use birdcage::{Birdcage, Exception, Sandbox};
use phylum_lockfile::generator::Generator;
use phylum_lockfile::{LockfileFormat, Package, PackageVersion, Parse, ThirdPartyVersion};
use phylum_types::types::package::{PackageDescriptor, PackageDescriptorAndLockfile};

use crate::commands::extensions::permissions;
use crate::commands::{CommandResult, ExitCode};
use crate::{config, dirs, print_user_warning};

pub struct ParsedLockfile {
    pub packages: Vec<PackageDescriptorAndLockfile>,
    pub format: LockfileFormat,
    pub path: PathBuf,
}

impl ParsedLockfile {
    fn new(path: PathBuf, format: LockfileFormat, packages: Vec<PackageDescriptor>) -> Self {
        let packages = packages
            .into_iter()
            .map(|package_descriptor| PackageDescriptorAndLockfile {
                package_descriptor,
                lockfile: Some(path.to_string_lossy().into_owned()),
            })
            .collect();

        Self { packages, format, path }
    }
}

pub fn lockfile_types(add_auto: bool) -> Vec<&'static str> {
    let mut lockfile_types = LockfileFormat::iter().map(|format| format.name()).collect::<Vec<_>>();

    // Add generic lockfile type.
    if add_auto {
        lockfile_types.push("auto");
    }

    lockfile_types
}

pub fn handle_parse(matches: &clap::ArgMatches) -> CommandResult {
    let sandbox_generation = !matches.get_flag("skip-sandbox");
    let project = phylum_project::get_current_project();
    let project_root = project.as_ref().map(|p| p.root());
    let lockfiles = config::lockfiles(matches, project.as_ref())?;

    let mut pkgs: Vec<PackageDescriptorAndLockfile> = Vec::new();
    for lockfile in lockfiles {
        let mut parsed_lockfile = parse_lockfile(
            &lockfile.path,
            project_root,
            Some(&lockfile.lockfile_type),
            sandbox_generation,
        )
        .with_context(|| format!("could not parse lockfile: {}", lockfile.path.display()))?;
        pkgs.append(&mut parsed_lockfile.packages);
    }

    serde_json::to_writer_pretty(&mut io::stdout(), &pkgs)?;

    Ok(ExitCode::Ok)
}

/// Parse a package lockfile.
pub fn parse_lockfile(
    path: impl Into<PathBuf>,
    project_root: Option<&PathBuf>,
    lockfile_type: Option<&str>,
    sandbox_generation: bool,
) -> Result<ParsedLockfile> {
    // Try and determine lockfile format.
    let path = path.into();
    let format = find_lockfile_format(&path, lockfile_type);

    // Attempt to parse with all known parsers as fallback.
    let (format, lockfile) = match format {
        Some(format) => format,
        None => return try_get_packages(path, project_root),
    };

    // Parse with the identified parser.
    let parser = format.parser();

    // Attempt to parse the identified lockfile.
    let mut lockfile_error = None;
    if let Some(lockfile) = lockfile {
        // Parse lockfile content.
        let content = fs::read_to_string(&lockfile).map_err(Into::into);
        let packages = content.and_then(|content| parse_lockfile_content(&content, parser));

        // Attempt to strip root path for identified lockfile
        let lockfile = strip_root_path(lockfile, project_root)?;

        match packages {
            Ok(packages) => return Ok(ParsedLockfile::new(lockfile, format, packages)),
            // Store error on failure.
            Err(err) => lockfile_error = Some(err),
        }
    }

    // If the path is neither a valid manifest nor lockfile, we abort.
    if !parser.is_path_manifest(&path) {
        // Return the original lockfile parsing error.
        match lockfile_error {
            Some(err) => return Err(err),
            // If it's neither manifest nor lockfile, `try_get_packages` is used instead.
            None => unreachable!("neither lockfile nor manifest"),
        }
    }

    // If the lockfile couldn't be parsed, or there is none, we generate a new one.

    // Find the generator for this lockfile format.
    let generator = match parser.generator() {
        Some(generator) => generator,
        None => return Err(anyhow!("unsupported manifest file {path:?}")),
    };

    let display_path = strip_root_path(path.to_path_buf(), project_root)?;

    eprintln!("Generating lockfile for manifest {display_path:?} using {format:?}…");

    // Generate a new lockfile.
    let generated_lockfile = generate_lockfile(generator, &path, sandbox_generation)?;

    // Parse the generated lockfile.
    let packages = parse_lockfile_content(&generated_lockfile, parser)?;

    Ok(ParsedLockfile::new(display_path, format, packages))
}

/// Generate a lockfile from a manifest inside a sandbox.
fn generate_lockfile(generator: &dyn Generator, path: &Path, sandbox: bool) -> Result<String> {
    let canonical_path = path.canonicalize()?;

    // Enable the sandbox.
    if sandbox {
        let birdcage = lockfile_generation_sandbox(&canonical_path)?;
        birdcage.lock()?;
    }

    let generated_lockfile = generator
        .generate_lockfile(&canonical_path)
        .context("Lockfile generation failed! For details, see: \
            https://docs.phylum.io/docs/lockfile_generation")?;

    Ok(generated_lockfile)
}

/// Create sandbox with exceptions allowing generation of any lockfile.
fn lockfile_generation_sandbox(canonical_manifest_path: &Path) -> Result<Birdcage> {
    let mut birdcage = permissions::default_sandbox()?;

    // Allow all networking.
    birdcage.add_exception(Exception::Networking)?;

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
    permissions::add_exception(&mut birdcage, Exception::Read(home.join(".rustup")))?;
    permissions::add_exception(&mut birdcage, Exception::Read(home.join(".cargo")))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/etc/passwd".into()))?;
    // Bundle.
    permissions::add_exception(&mut birdcage, Exception::Read("/dev/urandom".into()))?;
    // Maven.
    permissions::add_exception(&mut birdcage, Exception::Read("/opt/maven".into()))?;
    permissions::add_exception(&mut birdcage, Exception::Read("/etc/java-openjdk".into()))?;
    // Gradle.
    permissions::add_exception(
        &mut birdcage,
        Exception::Read("/usr/share/java/gradle/lib".into()),
    )?;
    // Pnpm.
    permissions::add_exception(&mut birdcage, Exception::Read("/tmp".into()))?;
    // Yarn.
    permissions::add_exception(&mut birdcage, Exception::Read(home.join("./yarn")))?;

    Ok(birdcage)
}

/// Attempt to parse a lockfile.
fn parse_lockfile_content(
    content: &str,
    parser: &'static dyn Parse,
) -> Result<Vec<PackageDescriptor>> {
    let packages = parser.parse(content).context("Failed to parse lockfile")?;
    Ok(filter_packages(packages))
}

/// Find a lockfile's format.
fn find_lockfile_format(
    path: &Path,
    lockfile_type: Option<&str>,
) -> Option<(LockfileFormat, Option<PathBuf>)> {
    // Determine format from lockfile type.
    if let Some(lockfile_type) = lockfile_type.filter(|lockfile_type| lockfile_type != &"auto") {
        let format = lockfile_type.parse::<LockfileFormat>().unwrap();

        // Skip lockfile analysis when path is only valid manifest.
        let parser = format.parser();
        let lockfile =
            (!parser.is_path_manifest(path) || parser.is_path_lockfile(path)).then(|| path.into());

        return Some((format, lockfile));
    }

    // Determine format based on lockfile path.
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

/// Attempt to get packages from an unknown lockfile type
fn try_get_packages(path: PathBuf, project_root: Option<&PathBuf>) -> Result<ParsedLockfile> {
    let data = fs::read_to_string(&path)?;
    let lockfile = strip_root_path(path, project_root)?;

    for format in LockfileFormat::iter() {
        let parser = format.parser();
        if let Some(packages) = parser.parse(data.as_str()).ok().filter(|pkgs| !pkgs.is_empty()) {
            log::info!("Identified lockfile type: {}", format);

            let packages = filter_packages(packages);

            return Ok(ParsedLockfile::new(lockfile, format, packages));
        }
    }

    Err(anyhow!("Failed to identify type for lockfile {lockfile:?}"))
}

/// Filter packages for submission.
fn filter_packages(mut packages: Vec<Package>) -> Vec<PackageDescriptor> {
    packages
        .drain(..)
        .filter_map(|package| {
            // Check if package should be submitted based on version format.
            let version = match package.version {
                PackageVersion::FirstParty(version) => version,
                PackageVersion::ThirdParty(ThirdPartyVersion { registry, version }) => {
                    log::debug!("Using registry {registry:?} for {} ({version})", package.name);
                    version
                },
                PackageVersion::Git(url) => {
                    log::debug!("Git dependency {} will not be analyzed ({url:?})", package.name);
                    url
                },
                PackageVersion::Path(path) => {
                    log::debug!("Ignoring filesystem dependency {} ({path:?})", package.name);
                    return None;
                },
                PackageVersion::DownloadUrl(url) => {
                    log::debug!("Ignoring remote dependency {} ({url:?})", package.name);
                    return None;
                },
            };

            Some(PackageDescriptor {
                package_type: package.package_type,
                version,
                name: package.name,
            })
        })
        .collect()
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
fn strip_root_path(path: PathBuf, project_root: Option<&PathBuf>) -> Result<PathBuf> {
    let base: Cow<'_, Path> = match project_root {
        Some(p) => p.into(),
        None => env::current_dir()?.into(),
    };

    relative_path(&base, &path)
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

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use super::*;

    #[test]
    fn it_can_identify_lock_file_types() {
        let test_cases = [
            ("../tests/fixtures/Gemfile.lock", LockfileFormat::Gem),
            ("../tests/fixtures/yarn-v1.lock", LockfileFormat::Yarn),
            ("../tests/fixtures/yarn.lock", LockfileFormat::Yarn),
            ("../tests/fixtures/package-lock.json", LockfileFormat::Npm),
            ("../tests/fixtures/package-lock-v6.json", LockfileFormat::Npm),
            ("../tests/fixtures/packages.lock.json", LockfileFormat::NugetLock),
            ("../tests/fixtures/gradle.lockfile", LockfileFormat::Gradle),
            ("../tests/fixtures/effective-pom.xml", LockfileFormat::Maven),
            ("../tests/fixtures/workspace-effective-pom.xml", LockfileFormat::Maven),
            ("../tests/fixtures/requirements-locked.txt", LockfileFormat::Pip),
            ("../tests/fixtures/Pipfile.lock", LockfileFormat::Pipenv),
            ("../tests/fixtures/poetry.lock", LockfileFormat::Poetry),
            ("../tests/fixtures/poetry_v2.lock", LockfileFormat::Poetry),
            ("../tests/fixtures/go.sum", LockfileFormat::Go),
            ("../tests/fixtures/Cargo_v1.lock", LockfileFormat::Cargo),
            ("../tests/fixtures/Cargo_v2.lock", LockfileFormat::Cargo),
            ("../tests/fixtures/Cargo_v3.lock", LockfileFormat::Cargo),
            ("../tests/fixtures/spdx-2.2.spdx", LockfileFormat::Spdx),
            ("../tests/fixtures/spdx-2.2.spdx.json", LockfileFormat::Spdx),
            ("../tests/fixtures/spdx-2.3.spdx.json", LockfileFormat::Spdx),
            ("../tests/fixtures/spdx-2.3.spdx.yaml", LockfileFormat::Spdx),
            ("../tests/fixtures/bom.1.3.json", LockfileFormat::CycloneDX),
            ("../tests/fixtures/bom.1.3.xml", LockfileFormat::CycloneDX),
            ("../tests/fixtures/bom.json", LockfileFormat::CycloneDX),
            ("../tests/fixtures/bom.xml", LockfileFormat::CycloneDX),
        ];

        for (file, expected_format) in test_cases {
            let parsed = try_get_packages(PathBuf::from(file), None).unwrap();
            assert_eq!(parsed.format, expected_format, "{}", file);
        }
    }

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
