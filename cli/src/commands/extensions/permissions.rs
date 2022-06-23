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
    fn resolve_path(src_path: String) -> Result<PathBuf> {
        let path = PathBuf::from(&src_path);

        if !path.is_relative() {
            return Err(anyhow!("`{src_path}`: absolute paths are not allowed"));
        }

        if path.components().into_iter().any(|c| c == Component::ParentDir) {
            return Err(anyhow!("`{src_path}`: directory traversals are not allowed"));
        }

        path.canonicalize().map_err(|e| e.into())
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
