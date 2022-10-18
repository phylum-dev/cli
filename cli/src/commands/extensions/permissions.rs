#[cfg(unix)]
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;

use anyhow::{anyhow, Result};
#[cfg(unix)]
use birdcage::error::{Error as SandboxError, Result as SandboxResult};
#[cfg(unix)]
use birdcage::{Birdcage, Exception, Sandbox};
use deno_runtime::permissions::PermissionsOptions;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::dirs::{self, expand_home_path};

/// Resource permissions.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Permission {
    #[serde(deserialize_with = "deserialize_permission_paths")]
    List(Vec<String>),
    Boolean(bool),
}

impl Default for Permission {
    fn default() -> Self {
        Self::Boolean(false)
    }
}

impl Permission {
    // XXX In Deno, `Some(vec![])` actually means "allow all". We don't want empty
    // `Vec<String>` permissions to allow access to all resources, so we
    // manually convert these instances into `None`.
    pub fn get(&self) -> Option<&Vec<String>> {
        const EMPTY_VEC: &Vec<String> = &Vec::new();
        match &self {
            Self::List(list) if list.is_empty() => None,
            Self::List(list) => Some(list),
            Self::Boolean(true) => Some(EMPTY_VEC),
            Self::Boolean(false) => None,
        }
    }

    /// Get Birdcage sandbox exception resource paths.
    #[cfg(unix)]
    pub fn sandbox_paths(&self) -> Cow<'_, Vec<String>> {
        match self {
            Permission::List(paths) => Cow::Borrowed(paths),
            Permission::Boolean(true) => Cow::Owned(vec!["/".into()]),
            Permission::Boolean(false) => Cow::Owned(Vec::new()),
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

    pub fn subset_of(&self, parent: &Permission) -> Result<Permission> {
        match (parent, self) {
            // Child deny-all always succeeds, returning deny-all.
            (_, &Permission::Boolean(false)) => Ok(Permission::Boolean(false)),
            // Parent deny-all fails with all child permissions but deny-all.
            (&Permission::Boolean(false), _) => {
                Err(anyhow!("All permissions are denied by the manifest"))
            },
            // Parent allow-all always succeeds, returning the child's permissions.
            (&Permission::Boolean(true), child) => Ok(child.clone()),
            // Child allow-all fails with more restrictive parent permissions.
            (_, &Permission::Boolean(true)) => {
                Err(anyhow!("The requested permissions are denied by the manifest"))
            },
            // Parent set vs child set have to be validated.
            // This will error if child is not subset of parent, and return the child set otherwise.
            (&Permission::List(ref parent), &Permission::List(ref child)) => {
                Permission::check_paths_include_children(parent, child)
                    .map(|_| Permission::List(child.clone()))
                    .map_err(|mismatches| {
                        anyhow!(
                            "The following paths are denied by the manifest: {}",
                            mismatches.join(", ")
                        )
                    })
            },
        }
    }

    fn check_paths_include_children(
        parent: &[String],
        child: &[String],
    ) -> StdResult<(), Vec<String>> {
        let parent_paths = parent.iter().map(PathBuf::from).collect::<Vec<_>>();
        let child_paths = child.iter().map(PathBuf::from).collect::<Vec<_>>();

        // Find all paths in `child` that don't have a prefix in `parent`.
        let without_parent: Vec<_> = child_paths
            .iter()
            .filter_map(|child| {
                // Using PathBuf::starts_with rather than String::starts_with in order to get
                // the correct semantics.
                if parent_paths.iter().any(|p| child.starts_with(p)) {
                    None
                } else {
                    Some(child.to_string_lossy().to_string())
                }
            })
            .collect::<Vec<_>>();

        // The above list must be empty for all child paths to be a subset of the
        // parent.
        if !without_parent.is_empty() {
            Err(without_parent)
        } else {
            Ok(())
        }
    }
}

/// Deserializer for automatically resolving `~/` path prefix.
pub fn deserialize_permission_paths<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // Ensure field is a valid string.
    let mut paths = Vec::<String>::deserialize(deserializer)?;

    // Resolve `~/` home prefix.
    let home = dirs::home_dir().map_err(D::Error::custom)?;
    for path in &mut paths {
        *path = expand_home_path(path, &home).display().to_string();
    }

    Ok(paths)
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Permissions {
    #[serde(default)]
    pub read: Permission,
    #[serde(default)]
    pub write: Permission,
    #[serde(default)]
    pub env: Permission,
    #[serde(default)]
    pub run: Permission,
    #[serde(default, deserialize_with = "deserialize_net_permission")]
    pub net: Permission,
}

/// Deserialize network permissions.
fn deserialize_net_permission<'de, D>(deserializer: D) -> Result<Permission, D::Error>
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

    /// Build a sandbox matching the requested permissions.
    #[cfg(unix)]
    pub fn build_sandbox(&self) -> Result<Birdcage> {
        let mut birdcage = default_sandbox()?;

        for path in self.read.sandbox_paths().iter().map(PathBuf::from) {
            add_exception(&mut birdcage, Exception::Read(path))?;
        }
        for path in self.write.sandbox_paths().iter().map(PathBuf::from) {
            add_exception(&mut birdcage, Exception::Write(path))?;
        }
        for path in self.run.sandbox_paths().iter() {
            let absolute_path = resolve_bin_path(path);
            add_exception(&mut birdcage, Exception::ExecuteAndRead(absolute_path))?;
        }

        if self.net.get().is_some() {
            birdcage.add_exception(Exception::Networking)?;
        }

        Ok(birdcage)
    }

    pub fn subset_of(&self, other: &Permissions) -> Result<Permissions> {
        let err_ctx = |name: &'static str| move |e| anyhow!("Invalid {name} permissions: {}", e);

        Ok(Permissions {
            read: self.read.subset_of(&other.read).map_err(err_ctx("read"))?,
            write: self.write.subset_of(&other.write).map_err(err_ctx("write"))?,
            env: self.env.subset_of(&other.env).map_err(err_ctx("env"))?,
            run: self.run.subset_of(&other.run).map_err(err_ctx("run"))?,
            net: self.net.subset_of(&other.net).map_err(err_ctx("net"))?,
        })
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
            allow_sys: None,
            allow_ffi: None,
            allow_hrtime: false,
            prompt: false,
        }
    }
}

/// Construct sandbox with a set of pre-defined acceptable exceptions.
#[cfg(unix)]
pub fn default_sandbox() -> SandboxResult<Birdcage> {
    let mut birdcage = Birdcage::new()?;

    // Permit read access to lib for dynamic linking.
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/lib".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/lib32".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/libx32".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/lib64".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/lib".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/lib32".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/libx32".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/lib64".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/opt/homebrew".into()))?;
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/local".into()))?;

    // Allow `env` exec to resolve binary paths.
    add_exception(&mut birdcage, Exception::ExecuteAndRead("/usr/bin/env".into()))?;

    // Allow access to DNS list.
    //
    // While this is required to send DNS requests for network queries, this does
    // not automatically allow any network access.
    add_exception(&mut birdcage, Exception::Read("/etc/resolv.conf".into()))?;

    // Allow reading SSL certificates.
    add_exception(&mut birdcage, Exception::Read("/etc/ca-certificates".into()))?;
    add_exception(&mut birdcage, Exception::Read("/etc/ssl".into()))?;

    // Allow executing anything in `$PATH`.
    //
    // While this is a quite wide-reaching exception, it should be safe considering
    // that the directories in `$PATH` shouldn't contain any sensitive data and
    // the remaining sandbox restrictions still applies.
    //
    // This is required since many package manager's build scripts will use various
    // executables in their build scripts.
    for path in env::var("PATH").iter().flat_map(|path| path.split(':')) {
        add_exception(&mut birdcage, Exception::ExecuteAndRead(path.into()))?;
    }

    Ok(birdcage)
}

/// Add an execption to the sandbox, ignoring invalid path errors.
#[cfg(unix)]
pub fn add_exception(birdcage: &mut Birdcage, exception: Exception) -> SandboxResult<()> {
    match birdcage.add_exception(exception) {
        Ok(_) => Ok(()),
        // Ignore invalid path errors.
        Err(SandboxError::InvalidPath(_)) => Ok(()),
        Err(err) => Err(err),
    }
}

/// Resolve non-absolute bin paths from `$PATH`.
pub fn resolve_bin_path<P: AsRef<Path>>(bin: P) -> PathBuf {
    let bin: &Path = bin.as_ref();

    // Do not transform absolute paths.
    if bin.has_root() {
        return bin.to_owned();
    }

    // Try to read `$PATH`.
    let path = match env::var("PATH") {
        Ok(path) => path,
        Err(_) => return PathBuf::from(bin),
    };

    // Return first path in `$PATH` that contains `bin`.
    for path in path.split(':') {
        let combined = PathBuf::from(path).join(&bin);
        if combined.exists() {
            return combined;
        }
    }

    PathBuf::from(bin)
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

        result.unwrap_err();
    }

    #[test]
    fn deserialize_bool_permissions() {
        let toml = "read = true\nnet = false";

        let permissions = toml::from_str::<Permissions>(toml).unwrap();

        assert_eq!(permissions.read.get(), Some(&Vec::new()));
        assert_eq!(permissions.net.get(), None);
    }

    #[test]
    fn paths_subset_algorithm() {
        // Shorthand to invoke Permission::paths_subset through &str slices.
        let paths_subset = |a: &[&str], b: &[&str]| {
            Permission::check_paths_include_children(
                &a.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                &b.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
        };

        // {} << A.
        assert!(paths_subset(&["/tmp"], &[]).is_ok());

        // A << A.
        assert!(paths_subset(&["/tmp"], &["/tmp"]).is_ok());
        assert!(paths_subset(&["/etc", "/tmp"], &["/tmp", "/etc"]).is_ok());
        assert!(paths_subset(&["/etc", "/tmp", "/"], &["/tmp", "/", "/etc"]).is_ok());

        // A << B if A = {a}, B = {b} and a < b.
        assert!(paths_subset(&["/"], &["/tmp"]).is_ok());
        assert!(paths_subset(&["/tmp"], &["/tmp/something"]).is_ok());

        // Not A << B if A = {a}, B = {b} and not a < b.
        assert!(paths_subset(&["/tmp"], &["/"]).is_err());
        assert!(paths_subset(&["/tmp"], &["/etc/something"]).is_err());

        // A << B if for each a in A, there exist at least one b in B such that a < b.
        assert!(paths_subset(&["/tmp", "/etc"], &["/etc/something"]).is_ok());
        assert!(paths_subset(&["/tmp", "/etc"], &["/etc", "/tmp/something"]).is_ok());
        assert!(paths_subset(&["/tmp", "/etc"], &["/tmp", "/etc/something"]).is_ok());
        assert!(paths_subset(&["/tmp", "/etc"], &["/etc/something", "/tmp/something"]).is_ok());

        // Not A << B if there exists one a in A such that for each b in B, not a < b.
        assert!(paths_subset(&["/tmp", "/etc"], &["/something"]).is_err());
        assert!(paths_subset(&["/tmp", "/etc"], &["/tmp", "/etc", "/something"]).is_err());
        assert!(paths_subset(&["/tmp", "/etc"], &["/tmp/a", "/etc/b", "/something"]).is_err());
    }

    #[test]
    fn permission_is_subset() {
        // Check that two Permission::List have the same content.
        fn permission_matches(permission: &Permission, content: &[&str]) -> bool {
            use std::collections::HashSet;

            if let Permission::List(l) = permission {
                l.iter().map(|s| s.as_str()).collect::<HashSet<_>>()
                    == content.iter().cloned().collect::<HashSet<_>>()
            } else {
                false
            }
        }

        // Shorthand to construct a Permission::List from a &str slice.
        fn permission_list(paths: &[&str]) -> Permission {
            Permission::List(paths.iter().cloned().map(String::from).collect())
        }

        // Test permission sets where both child and parent are lists.

        let parent = permission_list(&["/tmp", "/home/foo/.npm"]);
        let child = permission_list(&["/tmp/foo", "/home/foo/.npm/_cacache"]);
        assert!(permission_matches(&child.subset_of(&parent).unwrap(), &[
            "/tmp/foo",
            "/home/foo/.npm/_cacache"
        ]));

        let parent = permission_list(&["/etc", "/home/foo/.npm"]);
        let child = permission_list(&["/tmp/foo", "/home/foo/.npm/_cacache"]);
        assert!(child.subset_of(&parent).is_err());

        // Test permission sets where child is boolean.

        let parent = permission_list(&["/tmp", "/home/foo/.npm"]);
        let child = Permission::Boolean(true);
        assert!(&child.subset_of(&parent).is_err());

        let parent = permission_list(&["/tmp", "/home/foo/.npm"]);
        let child = Permission::Boolean(false);
        assert!(matches!(&child.subset_of(&parent), Ok(Permission::Boolean(false))));

        // Test permission sets where parent is boolean.

        let parent = Permission::Boolean(true);
        let child = permission_list(&["/tmp", "/home/foo/.npm"]);
        assert!(permission_matches(&child.subset_of(&parent).unwrap(), &[
            "/tmp",
            "/home/foo/.npm"
        ]));

        let parent = Permission::Boolean(false);
        let child = permission_list(&["/tmp", "/home/foo/.npm"]);
        assert!(&child.subset_of(&parent).is_err());

        // Test boolean permissions.

        let parent = Permission::Boolean(false);
        let child = Permission::Boolean(true);
        assert!(&child.subset_of(&parent).is_err());

        let parent = Permission::Boolean(true);
        let child = Permission::Boolean(false);
        assert!(matches!(&child.subset_of(&parent), Ok(Permission::Boolean(false))));

        let parent = Permission::Boolean(true);
        let child = Permission::Boolean(true);
        assert!(matches!(&child.subset_of(&parent), Ok(Permission::Boolean(true))));

        let parent = Permission::Boolean(false);
        let child = Permission::Boolean(false);
        assert!(matches!(&child.subset_of(&parent), Ok(Permission::Boolean(false))));
    }
}
