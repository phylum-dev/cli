use std::path::PathBuf;

use anyhow::{anyhow, Result};
use deno_runtime::permissions::PermissionsOptions;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

/// Resource permissions.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Permission {
    List(Vec<String>),
    Boolean(bool),
}

impl Default for Permission {
    fn default() -> Self {
        Self::Boolean(false)
    }
}

impl Permission {
    // XXX In Deno, `Some(vec![])` actually means "allow all". We never
    // want that, so we manually convert those instances into `None` in the
    // getter methods below. We need to make sure to always go through these when
    // constructing a `PermissionsOptions` object.
    pub fn get(&self) -> Option<&Vec<String>> {
        const EMPTY_VEC: &Vec<String> = &Vec::new();
        match &self {
            Self::List(list) if list.is_empty() => None,
            Self::List(list) => Some(list),
            Self::Boolean(true) => Some(EMPTY_VEC),
            Self::Boolean(false) => None,
        }
    }

    /// Check if access to resource is permitted.
    pub fn validate(&self, resource: &String, resource_type: &str) -> Result<()> {
        if self.get().map_or(false, |allowed| allowed.contains(resource)) {
            Ok(())
        } else {
            Err(anyhow!("Requires {resource_type} access to {resource:?}"))
        }
    }
}

#[derive(Clone, Default, Deserialize, Debug, Serialize)]
pub struct Permissions {
    #[serde(default)]
    pub read: Permission,
    #[serde(default)]
    pub write: Permission,
    #[serde(default)]
    pub env: Permission,
    #[serde(default)]
    pub run: Permission,
    #[serde(default, deserialize_with = "net_permission")]
    pub net: Permission,
}

/// Deserialize network permissions.
fn net_permission<'de, D>(deserializer: D) -> Result<Permission, D::Error>
where
    D: Deserializer<'de>,
{
    let permission = Permission::deserialize(deserializer)?;

    let net = match &permission {
        Permission::List(net) => net,
        _ => return Ok(permission),
    };

    // Error out if URL contains scheme or path segments.
    for url in net {
        if url.contains('/') {
            let err = format!(
                "Found '/' in net permission {url:?}, only domains and subdomains may be specified"
            );
            return Err(D::Error::custom(err));
        }
    }

    Ok(permission)
}

impl Permissions {
    pub fn is_allow_none(&self) -> bool {
        self.read.get().is_none()
            && self.write.get().is_none()
            && self.env.get().is_none()
            && self.run.get().is_none()
            && self.net.get().is_none()
    }
}

impl From<&Permissions> for PermissionsOptions {
    fn from(value: &Permissions) -> Self {
        let allow_read =
            value.read.get().map(|read| read.iter().map(PathBuf::from).collect::<Vec<_>>());

        let allow_write =
            value.write.get().map(|write| write.iter().map(PathBuf::from).collect::<Vec<_>>());

        let allow_env = value.env.get().cloned();
        let allow_net = value.net.get().cloned();
        let allow_run = value.run.get().cloned();

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
            read: Permission::List(vec![]),
            write: Permission::List(vec![]),
            env: Permission::List(vec![]),
            run: Permission::List(vec![]),
            net: Permission::List(vec![]),
        };

        let permissions_options = PermissionsOptions::try_from(&permissions).unwrap();

        assert!(permissions.is_allow_none());
        assert!(permissions_options.allow_read.is_none());
        assert!(permissions_options.allow_write.is_none());
        assert!(permissions_options.allow_env.is_none());
        assert!(permissions_options.allow_run.is_none());
        assert!(permissions_options.allow_net.is_none());
    }

    #[test]
    fn deserialize_valid_permissions() {
        let valid_toml = r#"net = ["api.phylum.io"]"#;

        let permissions = toml::from_str::<Permissions>(valid_toml).unwrap();

        assert_eq!(permissions.net, Permission::List(vec!["api.phylum.io".into()]));
    }

    #[test]
    fn deserialize_invalid_net_permissions() {
        let invalid_toml = r#"net = ["https://api.phylum.io/test"]"#;

        let result = toml::from_str::<Permissions>(invalid_toml);

        assert!(result.is_err());
    }

    #[test]
    fn deserialize_bool_permissions() {
        let toml = "read = true\nnet = false";

        let permissions = toml::from_str::<Permissions>(toml).unwrap();

        assert_eq!(permissions.read.get(), Some(&Vec::new()));
        assert_eq!(permissions.net.get(), None);
    }
}
