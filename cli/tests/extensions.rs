use std::convert::TryFrom;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use phylum_cli::commands::extensions::*;
use rand::Rng;
use regex::Regex;

////////////////////////////////////////////////////////////////////////////////
// Acceptance criteria tests
////////////////////////////////////////////////////////////////////////////////

// When a user runs `phylum extension add .`, the extension in the current
// working directory should be installed.
#[test]
fn extension_is_installed_correctly() {
    let tmp_dir = TmpDir::new();
    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("extension")
        .arg("add")
        .arg(fixtures_path().join("sample-extension"))
        .assert();

    std::env::set_var("XDG_DATA_HOME", &tmp_dir);

    let installed_ext = Extension::load("sample-extension").unwrap();

    assert_eq!(installed_ext.name(), "sample-extension");

    let not_installed_ext = Extension::load("sample-other-extension");
    assert!(not_installed_ext.is_err());
}

// After a user installs a new extension, foobar, it should become available to
// the user under the phylum cli, e.g., running `phylum foobar` should execute
// the foobar extension.
#[test]
fn can_run_installed_extension() {
    let tmp_dir = TmpDir::new();
    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("extension")
        .arg("add")
        .arg(fixtures_path().join("sample-extension"))
        .assert();

    let cmd = Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("sample-extension")
        .assert();

    cmd.success();
}

// When a user installs a valid extension it should print a message indicating
// success. It should also print a quick guide on the extension to give the user
// some context on how the given extension works.
#[test]
fn successful_installation_prints_message() {
    todo!();
}

// When a user attempts to install an invalid extension, it should fail and
// inform the user as to why.
#[test]
fn unsuccessful_installation_prints_failure_message() {
    todo!();
}

// When a user runs `phylum extension remove <extensionName>` the extension
// should be entirely removed from the user system.
#[test]
fn extension_is_uninstalled_correctly() {
    let tmp_dir = TmpDir::new();
    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("extension")
        .arg("add")
        .arg(fixtures_path().join("sample-extension"))
        .assert();

    assert!(
        std::fs::read_dir(&tmp_dir)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>()
            .len()
            > 1
    );

    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("extension")
        .arg("remove")
        .arg(fixtures_path().join("sample-extension"))
        .assert();
    for i in std::fs::read_dir(&tmp_dir).unwrap() {
        println!("{:?}", i);
    }
    assert!(
        std::fs::read_dir(&tmp_dir)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>()
            .len()
            == 1
    );
}

// When a user runs phylum extension or phylum extension list a list of
// currently installed extensions, their versions and a short one sentence blurb
// on what the extension does should be shown in a table format.
#[test]
fn extension_list_should_emit_output() {
    let tmp_dir = TmpDir::new();
    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("extension")
        .arg("add")
        .arg(fixtures_path().join("sample-extension"))
        .assert();

    let cmd = Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", &tmp_dir)
        .arg("extension")
        .arg("list")
        .assert();

    let output = std::str::from_utf8(&cmd.get_output().stdout).unwrap();
    let re = Regex::new(r#"^sample-extension\s+This extension does a thing"#).unwrap();

    assert!(output.lines().find(|m| re.is_match(m)).is_some());
}

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous tests
////////////////////////////////////////////////////////////////////////////////

#[test]
fn valid_extension_is_loaded_correctly() {
    let ext = Extension::try_from(fixtures_path().join("sample-extension")).unwrap();

    assert_eq!(ext.name(), "sample-extension");
}

////////////////////////////////////////////////////////////////////////////////
// Utilities
////////////////////////////////////////////////////////////////////////////////

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

impl AsRef<OsStr> for TmpDir {
    fn as_ref(&self) -> &OsStr {
        &self.0.as_os_str()
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        // std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn tmp_path() -> PathBuf {
    project_root().join("target").join("tests-tmp")
}
