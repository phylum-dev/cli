use std::path::PathBuf;

use deno_runtime::permissions::PermissionsOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Debug, Serialize)]
pub struct Permissions {
    read: Option<Vec<String>>,
    write: Option<Vec<String>>,
    env: Option<Vec<String>>,
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

    pub fn env(&self) -> Option<&Vec<String>> {
        self.env.as_ref().and_then(|v| if v.is_empty() { None } else { Some(v) })
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
            && self.env().is_none()
            && self.run().is_none()
            && self.net().is_none()
    }
}

impl From<&Permissions> for PermissionsOptions {
    fn from(value: &Permissions) -> Self {
        let allow_read =
            value.read().map(|read| read.iter().map(PathBuf::from).collect::<Vec<_>>());

        let allow_write =
            value.write().map(|write| write.iter().map(PathBuf::from).collect::<Vec<_>>());

        let allow_env = value.env().cloned();
        let allow_net = value.net().cloned();
        let allow_run = value.run().cloned();

        PermissionsOptions {
            allow_read,
            allow_write,
            allow_net,
            allow_run,
            allow_env,
            allow_ffi: None,
            allow_hrtime: false,
            prompt: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_vecs_are_turned_into_none() {
        let permissions = Permissions {
            read: Some(vec![]),
            write: Some(vec![]),
            env: Some(vec![]),
            run: Some(vec![]),
            net: Some(vec![]),
        };

        let permissions_options = PermissionsOptions::try_from(&permissions).unwrap();

        assert!(permissions.is_allow_none());
        assert!(permissions_options.allow_read.is_none());
        assert!(permissions_options.allow_write.is_none());
        assert!(permissions_options.allow_env.is_none());
        assert!(permissions_options.allow_run.is_none());
        assert!(permissions_options.allow_net.is_none());
    }
}
