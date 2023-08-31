use predicates::prelude::*;
use tempfile::NamedTempFile;

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
        .run(["sandbox", "--allow-run", "/", "bash", "-c", &format!("echo x > {test_file_path}")])
        .failure()
        .stderr(predicate::str::contains(expected_error));

    // Test read access.
    test_cli
        .run(["sandbox", "--allow-run", "cat", "cat", test_file_path])
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
        .run([
            "sandbox",
            "--allow-run",
            "/",
            "--allow-write",
            test_file_path,
            "bash",
            "-c",
            &format!("echo x > {test_file_path}"),
        ])
        .success();

    // Test read access.
    test_cli
        .run([
            "sandbox",
            "--allow-run",
            "cat",
            "--allow-read",
            test_file_path,
            "cat",
            test_file_path,
        ])
        .success();
}

#[test]
fn default_deny_env() {
    let test_cli = TestCli::builder().build();

    test_cli
        .cmd()
        .args(["sandbox", "env"])
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
        .args(["sandbox", "--allow-run", "/", "--allow-env", "TEST", "env"])
        .env("TEST", "VALUE")
        .assert()
        .success()
        .stdout(predicate::str::contains("TEST=VALUE"));
}

#[test]
fn default_deny_net() {
    let test_cli = TestCli::builder().build();

    test_cli
        .run(["sandbox", "--allow-run", "/", "--allow-env", "--", "curl", "http://phylum.io"])
        .failure()
        .stderr(predicate::str::contains("Could not resolve host: phylum.io"));
}

#[test]
fn allow_net() {
    let test_cli = TestCli::builder().build();

    test_cli
        .run([
            "sandbox",
            "--allow-run",
            "/",
            "--allow-env",
            "--allow-net",
            "curl",
            "http://phylum.io",
        ])
        .success();
}
