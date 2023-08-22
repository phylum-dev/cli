use std::borrow::Cow;
use std::convert::TryFrom;
use std::fs::{self, DirBuilder};
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
use crate::{deno, dirs, fs_compare};

const MANIFEST_NAME: &str = "PhylumExt.toml";

lazy_static! {
    static ref EXTENSION_NAME_RE: Regex = Regex::new(r#"^[a-z][a-z0-9-]+$"#).unwrap();
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Extension {
    /// Absolute path to the extension.
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
        if self.installed() {
            return Err(anyhow!("extension path and installation path are identical, skipping"));
        }

        let target_prefix = extension_path(self.name())?;

        if target_prefix.exists() {
            fs::remove_dir_all(&target_prefix)?;
        }

        self.copy_to(target_prefix)?;

        Ok(())
    }

    pub fn uninstall(self) -> Result<()> {
        println!("Uninstalling extension {}...", self.name());

        if !self.installed() {
            return Err(anyhow!("extension {} is not installed, skipping", self.name()));
        }

        if let Some(state_path) = self.state_path() {
            // Ignore errors since this may not exist
            let _ = fs::remove_dir_all(state_path);
        }

        fs::remove_dir_all(&self.path)?;

        println!("Extension {} uninstalled successfully", self.name());

        Ok(())
    }

    /// Return true if this is an installed extension.
    fn installed(&self) -> bool {
        let installed_path = extension_path(self.name())
            .ok()
            .and_then(|installed_path| installed_path.canonicalize().ok());

        Some(&self.path) == installed_path.as_ref()
    }

    /// Load an extension from the default path.
    pub fn load(name: &str) -> Result<Extension, anyhow::Error> {
        Extension::try_from(extension_path(name)?)
    }

    /// Return the path to this extension.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// A directory where installed extensions can store state.
    pub fn state_path(&self) -> Option<PathBuf> {
        self.installed().then(|| extension_state_path(self.name()).ok()).flatten()
    }

    /// Return the path to this extension's entry point.
    pub fn entry_point(&self) -> PathBuf {
        self.path.join(self.manifest.entry_point())
    }

    /// Execute an extension subcommand.
    pub async fn run(
        self,
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
                && fs_compare::dir_compare(&self.path, &other.path).unwrap_or(false))
    }
}

// Load the extension from the specified path.
impl TryFrom<PathBuf> for Extension {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        // Ensure that the path is an absolute path.
        let path = path.canonicalize()?;

        if !path.is_dir() {
            return Err(anyhow!("{}: not a directory", path.display()));
        }

        let manifest_path = path.join(MANIFEST_NAME);
        if !manifest_path.exists() {
            return Err(anyhow!("{}: missing {}", path.display(), MANIFEST_NAME));
        }

        let buf = fs::read_to_string(manifest_path)?;

        let manifest: ExtensionManifest = toml::from_str(&buf)?;
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
pub fn extensions_path() -> Result<PathBuf> {
    Ok(dirs::data_dir()?.join("phylum").join("extensions"))
}

fn extension_path(name: &str) -> Result<PathBuf> {
    Ok(extensions_path()?.join(name))
}

fn extension_state_path(name: &str) -> Result<PathBuf> {
    Ok(dirs::state_dir()?.join("phylum/extensions").join(name))
}
