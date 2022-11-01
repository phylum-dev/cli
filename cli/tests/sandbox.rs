use predicates::prelude::*;
use tempfile::{NamedTempFile, TempDir};

use crate::common::TestCli;

#[test]
fn default_deny_fs() {
    let test_file = NamedTempFile::new().unwrap();
    let test_file_path = test_file.path().to_str().unwrap();
    let test_cli = TestCli::builder().build();

    #[cfg(target_os = "linux")]
    let expected_error = "Permission denied";
    #[cfg(not(target_os = "linux"))]
    let expected_error = "Operation not permitted";

    // Test write access.
    test_cli
        .run(&["sandbox", "bash", "-c", &format!("echo x > {test_file_path}")])
        .failure()
        .stderr(predicate::str::contains(expected_error));

    // Test read access.
    test_cli
        .run(&["sandbox", "cat", test_file_path])
        .failure()
        .stderr(predicate::str::contains(expected_error));
}

#[test]
fn allow_fs() {
    let test_file = NamedTempFile::new().unwrap();
    let test_file_path = test_file.path().to_str().unwrap();
    let test_cli = TestCli::builder().build();

    // Test write access.
    test_cli
        .run(&[
            "sandbox",
            "--allow-write",
            &test_file_path,
            "bash",
            "-c",
            &format!("echo x > {test_file_path}"),
        ])
        .success();

    // Test read access.
    test_cli.run(&["sandbox", "--allow-read", &test_file_path, "cat", &test_file_path]).success();
}

#[test]
fn default_deny_env() {
    let test_cli = TestCli::builder().build();

    test_cli
        .cmd()
        .args(&["sandbox", "env"])
        .env("TEST", "VALUE")
        .assert()
        .success()
        .stdout(predicate::str::contains("TEST=VALUE").not());
}

#[test]
fn allow_env() {
    let test_cli = TestCli::builder().build();

    test_cli
        .cmd()
        .args(&["sandbox", "--allow-env", "TEST", "env"])
        .env("TEST", "VALUE")
        .assert()
        .success()
        .stdout(predicate::str::contains("TEST=VALUE"));
}

#[test]
fn default_deny_net() {
    let test_cli = TestCli::builder().build();

    test_cli
        .run(&["sandbox", "--allow-env", "--", "curl", "http://phylum.io"])
        .failure()
        .stderr(predicate::str::contains("Could not resolve host: phylum.io"));
}

#[test]
fn allow_net() {
    let test_cli = TestCli::builder().build();

    test_cli.run(&["sandbox", "--allow-env", "--allow-net", "curl", "http://phylum.io"]).success();
}

#[test]
fn error_exit() {
    let test_cli = TestCli::builder().build();
    let ipc_path = TempDir::new().unwrap();
    let ipc_path_error = ipc_path.path().join("error");

    test_cli
        .run(&["sandbox", "--ipc-path", ipc_path.path().to_str().unwrap(), "blargle"])
        .failure();

    // Test that the error IPC file exists, and that it contains an expected value.
    assert!(ipc_path_error.exists());
    assert!(std::fs::read_to_string(ipc_path_error).unwrap().contains("Sandbox"));
}
