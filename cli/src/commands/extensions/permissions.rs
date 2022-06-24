use std::path::{Component, PathBuf};

use anyhow::{anyhow, Result};
use deno_runtime::permissions::PermissionsOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Debug, Serialize)]
pub struct Permissions {
    read: Option<Vec<String>>,
    write: Option<Vec<String>>,
    run: Option<Vec<String>>,
    net: Option<Vec<String>>,
}

impl Permissions {
    pub fn is_empty(&self) -> bool {
        self.read.is_none() && self.write.is_none() && self.run.is_none() && self.net.is_none()
    }

    pub fn read(&self) -> Option<&Vec<String>> {
        self.read.as_ref()
    }

    pub fn write(&self) -> Option<&Vec<String>> {
        self.write.as_ref()
    }

    pub fn run(&self) -> Option<&Vec<String>> {
        self.run.as_ref()
    }

    pub fn net(&self) -> Option<&Vec<String>> {
        self.net.as_ref()
    }

    fn resolve_path<S: AsRef<str> + std::fmt::Display>(src_path: S) -> Result<PathBuf> {
        let path = PathBuf::from(src_path.as_ref());

        if !path.is_relative() {
            return Err(anyhow!("`{src_path}`: absolute paths are not allowed"));
        }

        if path.components().into_iter().any(|c| c == Component::ParentDir) {
            return Err(anyhow!("`{src_path}`: directory traversals are not allowed"));
        }

        // Path is intentionally not canonicalized. Checking for the existence
        // of the file the permission was requested for should be an extension's
        // responsibility.
        Ok(path)
    }
}

impl TryFrom<&Permissions> for PermissionsOptions {
    type Error = anyhow::Error;

    fn try_from(value: &Permissions) -> Result<Self, Self::Error> {
        let Permissions { read, write, net, run } = value;

        let allow_read = match read {
            Some(read) => {
                Some(read.iter().map(Permissions::resolve_path).collect::<Result<Vec<_>>>()?)
            },
            None => None,
        };

        let allow_write = match write {
            Some(write) => {
                Some(write.iter().map(Permissions::resolve_path).collect::<Result<Vec<_>>>()?)
            },
            None => None,
        };

        let allow_net = net.clone();
        let allow_run = run.clone();

        Ok(PermissionsOptions {
            allow_read,
            allow_write,
            allow_net,
            allow_run,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed_permission_is_converted() {
        let permissions = Permissions {
            read: Some(vec![
                "./node_modules".to_string(),
                "package-lock.json".to_string(),
                "yarn.lock".to_string(),
            ]),
            write: None,
            run: Some(vec![
                "npm".to_string(),
                "yarn".to_string(),
                "yarnpkg".to_string(),
                "pip".to_string(),
                "poetry".to_string(),
            ]),
            net: None,
        };
        let permissions_options = PermissionsOptions::try_from(&permissions);

        println!("{:?}", permissions_options);
        assert!(permissions_options.is_ok());
    }

    #[test]
    fn directory_traversal_is_denied() {
        let permissions = Permissions {
            read: Some(vec!["../node_modules".to_string()]),
            write: None,
            run: None,
            net: None,
        };
        let permissions_options = PermissionsOptions::try_from(&permissions);

        assert!(permissions_options.is_err());
    }

    #[test]
    fn absolute_paths_are_denied() {
        let permissions = Permissions {
            read: Some(vec!["/tmp/node_modules".to_string()]),
            write: None,
            run: None,
            net: None,
        };
        let permissions_options = PermissionsOptions::try_from(&permissions);

        assert!(permissions_options.is_err());
    }
}
