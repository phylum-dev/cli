use std::convert::TryFrom;
use std::path::{Path, PathBuf};

use phylum_cli::commands::extensions::*;
use rand::Rng;

#[test]
fn valid_extension_is_loaded_correctly() {
    let ext = Extension::try_from(fixtures_path().join("sample-extension")).unwrap();

    assert_eq!(ext.name(), "sample-extension");
}

#[test]
fn extension_is_installed_correctly() {
    let tmp_dir = TmpDir::new();
    std::env::set_var("XDG_DATA_HOME", tmp_dir.0.as_os_str());

    let ext = Extension::try_from(fixtures_path().join("sample-extension")).unwrap();
    ext.install().unwrap();

    let installed_ext = Extension::load(ext.name()).unwrap();
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn fixtures_path() -> PathBuf {
    project_root()
        .join("cli")
        .join("tests")
        .join("fixtures")
        .join("extensions")
}

struct TmpDir(PathBuf);

impl TmpDir {
    fn new() -> Self {
        let dir_name: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let path = tmp_path().join(dir_name);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl AsRef<Path> for TmpDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn tmp_path() -> PathBuf {
    project_root().join("target").join("tests-tmp")
}
