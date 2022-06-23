use std::path::{Component, PathBuf};

use anyhow::{anyhow, Result};
use deno_runtime::permissions::PermissionsOptions;
use serde::Deserialize;

#[derive(Clone, Default, Deserialize, Debug)]
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

    fn resolve_path(src_path: String) -> Result<PathBuf> {
        let path = PathBuf::from(&src_path);

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

impl TryFrom<Permissions> for PermissionsOptions {
    type Error = anyhow::Error;

    fn try_from(value: Permissions) -> Result<Self, Self::Error> {
        let Permissions { read, write, net, run } = value;

        let allow_read = match read {
            Some(read) => {
                Some(read.into_iter().map(Permissions::resolve_path).collect::<Result<Vec<_>>>()?)
            },
            None => None,
        };

        let allow_write = match write {
            Some(write) => {
                Some(write.into_iter().map(Permissions::resolve_path).collect::<Result<Vec<_>>>()?)
            },
            None => None,
        };

        let allow_net = net;
        let allow_run = run;

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
        let permissions_options: Result<PermissionsOptions> = Permissions {
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
        }
        .try_into();

        println!("{:?}", permissions_options);
        assert!(permissions_options.is_ok());
    }

    #[test]
    fn directory_traversal_is_denied() {
        let permissions_options: Result<PermissionsOptions> = Permissions {
            read: Some(vec!["../node_modules".to_string()]),
            write: None,
            run: None,
            net: None,
        }
        .try_into();

        assert!(permissions_options.is_err());
    }

    #[test]
    fn absolute_paths_are_denied() {
        let permissions_options: Result<PermissionsOptions> = Permissions {
            read: Some(vec!["/tmp/node_modules".to_string()]),
            write: None,
            run: None,
            net: None,
        }
        .try_into();

        assert!(permissions_options.is_err());
    }
}
