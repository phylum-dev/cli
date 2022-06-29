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

// XXX In Deno, `Some(vec![])` actually means "allow all". We never
// want that, so we manually convert those instances into `None` in the
// getter methods below. We need to make sure to always go through these when
// constructing a `PermissionsOptions` object.
impl Permissions {
    pub fn read(&self) -> Option<&Vec<String>> {
        self.read.as_ref().and_then(|v| if v.is_empty() { None } else { Some(v) })
    }

    pub fn write(&self) -> Option<&Vec<String>> {
        self.write.as_ref().and_then(|v| if v.is_empty() { None } else { Some(v) })
    }

    pub fn run(&self) -> Option<&Vec<String>> {
        self.run.as_ref().and_then(|v| if v.is_empty() { None } else { Some(v) })
    }

    pub fn net(&self) -> Option<&Vec<String>> {
        self.net.as_ref().and_then(|v| if v.is_empty() { None } else { Some(v) })
    }

    pub fn is_allow_none(&self) -> bool {
        self.read().is_none()
            && self.write().is_none()
            && self.run().is_none()
            && self.net().is_none()
    }
}

impl TryFrom<&Permissions> for PermissionsOptions {
    type Error = anyhow::Error;

    fn try_from(value: &Permissions) -> Result<Self, Self::Error> {
        let allow_read =
            value.read().map(|read| read.iter().map(PathBuf::from).collect::<Vec<_>>());

        let allow_write =
            value.write().map(|write| write.iter().map(PathBuf::from).collect::<Vec<_>>());

        let allow_net = value.net().cloned();
        let allow_run = value.run().cloned();

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
    fn empty_vecs_are_turned_into_none() {
        let permissions = Permissions {
            read: Some(vec![]),
            write: Some(vec![]),
            run: Some(vec![]),
            net: Some(vec![]),
        };

        let permissions_options = PermissionsOptions::try_from(&permissions).unwrap();

        assert!(permissions.is_allow_none());
        assert!(permissions_options.allow_read.is_none());
        assert!(permissions_options.allow_write.is_none());
        assert!(permissions_options.allow_run.is_none());
        assert!(permissions_options.allow_net.is_none());
    }
}
