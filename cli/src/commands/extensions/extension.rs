use std::borrow::Cow;
use std::convert::TryFrom;
use std::fs::{self, DirBuilder, File};
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use futures::future::BoxFuture;
use lazy_static::lazy_static;
use log::{warn, LevelFilter};
use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::api::PhylumApi;
use crate::commands::extensions::permissions::Permissions;
use crate::commands::CommandResult;
use crate::{deno, dirs};

const MANIFEST_NAME: &str = "PhylumExt.toml";

lazy_static! {
    static ref EXTENSION_NAME_RE: Regex = Regex::new(r#"^[a-z][a-z0-9-]+$"#).unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtensionManifest {
    name: String,
    description: Option<String>,
    entry_point: Option<String>,
    permissions: Option<Permissions>,
}

impl ExtensionManifest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Default::default(),
            entry_point: Default::default(),
            permissions: Default::default(),
        }
    }

    fn entry_point(&self) -> &str {
        self.entry_point.as_deref().unwrap_or("main.ts")
    }
}

#[derive(Debug)]
pub struct Extension {
    path: PathBuf,
    manifest: ExtensionManifest,
}

impl Extension {
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn description(&self) -> Option<&str> {
        self.manifest.description.as_deref()
    }

    pub fn permissions(&self) -> Cow<'_, Permissions> {
        match self.manifest.permissions.as_ref() {
            Some(permissions) => Cow::Borrowed(permissions),
            None => Cow::Owned(Permissions::default()),
        }
    }

    /// Copy the extension to a new path.
    fn copy_to<P: AsRef<Path>>(&self, dest: P) -> Result<()> {
        for entry in WalkDir::new(&self.path) {
            let source_path = entry?.into_path();
            let dest_path = dest.as_ref().join(source_path.strip_prefix(&self.path)?);

            if source_path.is_dir() {
                let mut builder = DirBuilder::new();

                #[cfg(unix)]
                builder.mode(0o700);

                builder.recursive(true);
                builder.create(&dest_path)?;
            } else if source_path.is_symlink() {
                warn!(
                    "install {}: `{:?}`: is a symbolic link, skipping",
                    self.manifest.name, source_path
                );
            } else if source_path.is_file() {
                if dest_path.exists() {
                    return Err(anyhow!("{}: already exists", dest_path.display()));
                } else {
                    fs::copy(source_path, dest_path)?;
                }
            }
        }
        Ok(())
    }

    /// Install the extension in the default path.
    pub fn install(&self) -> Result<()> {
        println!("Installing extension {}...", self.name());
        let target_prefix = extension_path(self.name())?;

        if target_prefix == self.path {
            return Err(anyhow!("extension path and installation path are identical, skipping"));
        }

        if target_prefix.exists() {
            fs::remove_dir_all(&target_prefix)?;
        }

        self.copy_to(target_prefix)?;

        println!("Extension {} installed successfully", self.name());

        Ok(())
    }

    pub fn uninstall(self) -> Result<()> {
        println!("Uninstalling extension {}...", self.name());
        let target_prefix = extension_path(self.name())?;

        if target_prefix != self.path {
            return Err(anyhow!("extension {} is not installed, skipping", self.name()));
        }

        fs::remove_dir_all(&self.path)?;

        println!("Extension {} uninstalled successfully", self.name());

        Ok(())
    }

    /// Load an extension from the default path.
    pub fn load(name: &str) -> Result<Extension, anyhow::Error> {
        Extension::try_from(extension_path(name)?)
    }

    /// Return the path to this extension.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Return the path to this extension's entry point.
    pub fn entry_point(&self) -> PathBuf {
        self.path.join(self.manifest.entry_point())
    }

    /// Execute an extension subcommand.
    pub async fn run(
        &self,
        api: BoxFuture<'static, Result<PhylumApi>>,
        args: Vec<String>,
    ) -> CommandResult {
        // Disable logging for running extensions.
        log::set_max_level(LevelFilter::Off);

        // Execute Deno extension.
        deno::run(api, self, args).await
    }
}

impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            || (self.name() == other.name()
                && dir_compare(&self.path, &other.path).unwrap_or(false))
    }
}

// Load the extension from the specified path.
impl TryFrom<PathBuf> for Extension {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.is_dir() {
            return Err(anyhow!("{}: not a directory", path.display()));
        }

        let manifest_path = path.join(MANIFEST_NAME);
        if !manifest_path.exists() {
            return Err(anyhow!("{}: missing {}", path.display(), MANIFEST_NAME));
        }

        let buf = fs::read(manifest_path)?;

        let manifest: ExtensionManifest = toml::from_slice(&buf)?;
        let entry_point_path = path.join(manifest.entry_point());

        if !entry_point_path.exists() {
            return Err(anyhow!("{}: entry point does not exist", entry_point_path.display()));
        }

        if !entry_point_path.is_file() {
            return Err(anyhow!("{}: entry point is not a file", entry_point_path.display()));
        }

        validate_name(&manifest.name)?;

        // TODO add further validation if necessary:
        // - Check that the entry point is a supported format (.wasm?)
        // - Check that the entry point is appropriately signed
        Ok(Extension { path, manifest })
    }
}

/// Check extension name for validity.
pub fn validate_name(name: &str) -> Result<(), anyhow::Error> {
    if EXTENSION_NAME_RE.is_match(name) {
        Ok(())
    } else {
        Err(anyhow!(
            "{}: invalid extension name, must start with a letter and can contain only lowercase \
             alphanumeric characters or dashes (-)",
            name
        ))
    }
}

// Construct and return the extension path: $XDG_DATA_HOME/phylum/extensions
pub fn extensions_path() -> Result<PathBuf, anyhow::Error> {
    Ok(dirs::data_dir()?.join("phylum").join("extensions"))
}

fn extension_path(name: &str) -> Result<PathBuf, anyhow::Error> {
    Ok(extensions_path()?.join(name))
}

/// Compare the contents of two directories
fn dir_compare<A: AsRef<Path>, B: AsRef<Path>>(a: A, b: B) -> Result<bool> {
    let a = WalkDir::new(a).sort_by_file_name();
    let b = WalkDir::new(b).sort_by_file_name();

    for (a, b) in a.into_iter().zip(b) {
        let a = a?;
        let b = b?;

        if a.depth() == 0 && b.depth() == 0 {
            // Don't check the top-level directory
            continue;
        }

        if a.depth() != b.depth()
            || a.file_name() != b.file_name()
            || a.file_type() != b.file_type()
        {
            log::trace!(
                "Directory structure mismatch:\n  {}\n  {}",
                a.path().display(),
                b.path().display()
            );

            return Ok(false);
        }

        if a.file_type().is_file() && !file_compare(a.into_path(), b.into_path())? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Compare the contents of two files
fn file_compare<A: AsRef<Path>, B: AsRef<Path>>(a: A, b: B) -> Result<bool> {
    log::trace!("Comparing files:\n  {}\n  {}", a.as_ref().display(), b.as_ref().display());
    let a = File::open(a)?;
    let b = File::open(b)?;

    if a.metadata()?.len() != b.metadata()?.len() {
        log::trace!("File length mismatch");
        return Ok(false);
    }

    let mut a = BufReader::new(a);
    let mut b = BufReader::new(b);

    loop {
        let a_buf = a.fill_buf()?;
        let b_buf = b.fill_buf()?;

        if a_buf.is_empty() && b_buf.is_empty() {
            // Both EOF at same time. Equal files
            return Ok(true);
        }

        let cmp_len = std::cmp::min(a_buf.len(), b_buf.len());
        if cmp_len == 0 {
            // Only 1 EOF. Files are different
            // Note: Because of the file length check above, this should be impossible
            //       except perhaps if a file is actively being changed.
            log::trace!("File length mismatch (race condition?)");
            return Ok(false);
        }

        if a_buf[..cmp_len] != b_buf[..cmp_len] {
            log::trace!("File contents mismatch");
            return Ok(false);
        }

        a.consume(cmp_len);
        b.consume(cmp_len);
    }
}
