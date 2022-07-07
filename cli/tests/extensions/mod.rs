#![cfg(feature = "extensions")]

use std::convert::TryFrom;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lazy_static::lazy_static;
use phylum_cli::commands::extensions::extension::Extension;
use test_utils::*;

mod module_loader;
mod permissions;

lazy_static! {
    // Lock this mutex when setting an environment variable, for the lifetime of function calls
    // depending on that specific environment variable value. Currently only used by the
    // `extension_is_installed_correctly` test. This trades some contention for the possibility
    // of running tests in parallel.
    static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
}

////////////////////////////////////////////////////////////////////////////////
// Acceptance criteria tests
////////////////////////////////////////////////////////////////////////////////

// When a user runs `phylum extension add .`, the extension in the current
// working directory should be installed.
#[test]
fn extension_is_installed_correctly() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("sample")).success();

    let _guard = ENV_MUTEX.lock().unwrap();
    env::set_var("XDG_DATA_HOME", test_cli.temp_path());

    let installed_ext = Extension::load("sample").unwrap();
    assert_eq!(installed_ext.name(), "sample");

    let not_installed_ext = Extension::load("sample-other");
    assert!(not_installed_ext.is_err());
}

// After a user installs a new extension, foobar, it should become available to
// the user under the phylum cli, e.g., running `phylum foobar` should execute
// the foobar extension.
#[test]
fn can_run_installed_extension() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("sample")).success();
    test_cli.run(&["sample"]).success().stdout("Hello, World!\n");
}

// When a user installs a valid extension it should print a message indicating
// success. It should also print a quick guide on the extension to give the user
// some context on how the given extension works.
#[test]
fn successful_installation_prints_message() {
    let test_cli = TestCli::builder().build();

    test_cli
        .install_extension(&fixtures_path().join("sample"))
        .success()
        .stdout(predicate::str::contains("Extension sample installed successfully"));
}

// When a user attempts to install an invalid extension, it should fail and
// inform the user as to why.
#[test]
fn unsuccessful_installation_prints_failure_message() {
    let test_cli = TestCli::builder().build();

    // Install the extension. Should succeed.
    test_cli.install_extension(&fixtures_path().join("sample")).success();

    // Reinstall the same extension. Should fail with an error.
    test_cli
        .install_extension(&fixtures_path().join("sample"))
        .failure()
        .stderr(predicate::str::contains("extension already exists"));

    // Try to install the extension from the installed path. Should fail with an
    // error.
    test_cli
        .install_extension(
            &test_cli.temp_path().to_owned().join("phylum").join("extensions").join("sample"),
        )
        .failure()
        .stderr(predicate::str::contains("skipping"));
}

// When a user runs `phylum extension remove <extensionName>` the extension
// should be entirely removed from the user system.
#[test]
fn extension_is_uninstalled_correctly() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("sample")).success();

    let extension_path =
        test_cli.temp_path().to_path_buf().join("phylum").join("extensions").join("sample");

    assert!(walkdir::WalkDir::new(&extension_path).into_iter().count() > 1);

    test_cli.run(&["extension", "uninstall", "sample"]).success();

    assert!(!extension_path.exists());
}

// When a user runs phylum extension or phylum extension list a list of
// currently installed extensions, their versions and a short one sentence blurb
// on what the extension does should be shown in a table format.
#[test]
fn extension_list_should_emit_output() {
    let test_cli = TestCli::builder().build();

    // Output that no extension is installed when that is the case.
    test_cli.run(&["extension", "list"]).success().stdout(predicate::str::contains("No extension"));

    // Install one extension.
    test_cli.install_extension(&fixtures_path().join("sample")).success();

    // Output name and description of the extension when one is installed
    test_cli.run(&["extension", "list"]).success().stdout(
        predicate::str::contains("sample")
            .and(predicate::str::contains("This extension does a thing")),
    );
}

// Extensions relying on the injected Phylum API work.
#[test]
fn injected_api() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("api")).success();
    test_cli.run(&["api"]).success().stdout("44\n");
}

// Extensions can access CLI arguments.
#[test]
fn arg_access() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("args")).success();
    test_cli
        .run(&["args", "--test", "-x", "a"])
        .success()
        .stdout(predicate::str::contains(r#"[ "--test", "-x", "a" ]"#));
}

// Extension creation works.
#[test]
fn create_extension() {
    let test_cli = TestCli::builder().cwd_temp().build();

    test_cli
        .run(&["extension", "new", "my-ext"])
        .success()
        .stderr(predicates::str::contains("✅ Extension created successfully"));
}

// Extension creation with invalid name fails
#[test]
fn create_incorrect_name() {
    let test_cli = TestCli::builder().cwd_temp().build();

    test_cli
        .run(&["extension", "new", "@@@"])
        .failure()
        .stderr(predicates::str::contains("invalid extension name"));
}

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous tests
////////////////////////////////////////////////////////////////////////////////

#[test]
fn valid_extension_is_loaded_correctly() {
    let ext = Extension::try_from(fixtures_path().join("sample")).unwrap();

    assert_eq!(ext.name(), "sample");
}

#[test]
fn conflicting_extension_name_is_filtered() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("ping")).success();
    test_cli
        .run(&["extension", "list"])
        .success()
        .stderr(predicate::str::contains("extension was filtered out"));
}

////////////////////////////////////////////////////////////////////////////////
// Utilities
////////////////////////////////////////////////////////////////////////////////

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
}

fn fixtures_path() -> PathBuf {
    project_root().join("cli").join("tests").join("fixtures").join("extensions")
}
