use std::{convert::TryFrom, fs::File, io::Read, path::PathBuf};

use anyhow::{anyhow, Result};
use serde::Deserialize;

const MANIFEST_NAME: &str = "PhylumExt.toml";

#[derive(Debug)]
pub struct Extension {
    path: PathBuf,
    manifest: ExtensionManifest,
}

#[derive(Deserialize, Debug)]
pub struct ExtensionManifest {
    name: String,
    description: Option<String>,
    entry_point: String,
}

impl Extension {
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn description(&self) -> Option<&str> {
        self.manifest.description.as_ref().map(String::as_str)
    }

    /// Install the extension in the default path.
    pub fn install(&self) -> Result<()> {
        let target_prefix = extensions_path()?.join(self.name());

        if target_prefix == self.path {
            return Err(anyhow!("extension path and installation path are identical, skipping"));
        }

        for entry in walkdir::WalkDir::new(&self.path) {
            let source_path = entry?.into_path();
            let dest_path = target_prefix.join(source_path.strip_prefix(&self.path)?);

            if source_path.is_dir() {
                std::fs::create_dir_all(dest_path)?;
            } else if source_path.is_file() {
                if dest_path.exists() {
                    return Err(anyhow!("{}: already exists", dest_path.to_string_lossy()));
                } else {
                    std::fs::copy(source_path, dest_path)?;
                }
            }
        }

        Ok(())
    }

    pub fn uninstall(self) -> Result<()> {
        let target_prefix = extensions_path()?.join(self.name());

        if target_prefix != self.path {
            return Err(anyhow!("extension is not installed, skipping"));
        }

        for entry in walkdir::WalkDir::new(&self.path).contents_first(true) {
            let entry = entry?.into_path();
            if entry.is_dir() {
                std::fs::remove_dir(entry)?;
            } else if entry.is_file() {
                std::fs::remove_file(entry)?;
            }
        }

        Ok(())
    }

    /// Load an extension from the default path.
    pub fn load(name: &str) -> Result<Extension, anyhow::Error> {
        Extension::try_from(extensions_path()?.join(name))
    }
}

// Load the extension from the specified path.
impl TryFrom<PathBuf> for Extension {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.is_dir() {
            return Err(anyhow!("{}: not a directory", path.to_string_lossy()));
        }

        let manifest_path = path.join(MANIFEST_NAME);
        if !manifest_path.exists() {
            return Err(anyhow!(
                "{}: missing {}",
                path.to_string_lossy(),
                MANIFEST_NAME
            ));
        }

        let mut buf = Vec::new();
        File::open(manifest_path)?.read_to_end(&mut buf)?;

        let manifest: ExtensionManifest = toml::from_slice(&buf)?;
        let entry_point_path = path.join(&manifest.entry_point);

        if !entry_point_path.exists() {
            return Err(anyhow!(
                "{}: entry point does not exist",
                entry_point_path.to_string_lossy()
            ));
        }

        if !entry_point_path.is_file() {
            return Err(anyhow!(
                "{}: entry point is not a file",
                entry_point_path.to_string_lossy()
            ));
        }

        // TODO add further validation if necessary:
        // - Check that the name matches /^[a-z0-9-_]+$/
        // - Check that the entry point is a supported format (.wasm?)
        // - Check that the entry point is appropriately signed
        Ok(Extension { path, manifest })
    }
}


// Construct and return the extension path: $XDG_DATA_HOME/phylum/extensions
pub fn extensions_path() -> Result<PathBuf, anyhow::Error> {
    Ok(crate::config::data_dir()?.join("phylum").join("extensions"))
}
