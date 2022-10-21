use predicates::prelude::*;
use tempfile::NamedTempFile;

use crate::common::TestCli;

#[test]
fn default_deny_fs() {
    let test_file = NamedTempFile::new().unwrap();
    let test_file_path = test_file.path().to_str().unwrap();
    let test_cli = TestCli::builder().build();

    // Test write access.
    test_cli
        .run(&["sandbox", "--allow-read", "./", "sh", "-c", &format!("echo x > {test_file_path}")])
        .failure()
        .stderr(predicate::str::contains("Permission denied"));

    // Test read access.
    test_cli
        .run(&["sandbox", "cat", test_file_path])
        .failure()
        .stderr(predicate::str::contains("Permission denied"));
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
            "--allow-read",
            "./",
            "--allow-write",
            &test_file_path,
            "sh",
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
        .args(&["sandbox", "--allow-read", "./", "sh", "-c", "echo $TEST"])
        .env("TEST", "VALUE")
        .assert()
        .success()
        .stdout("\n")
        .stderr("");
}

#[test]
fn allow_env() {
    let test_cli = TestCli::builder().build();

    test_cli
        .cmd()
        .args(&["sandbox", "--allow-read", "./", "--allow-env", "TEST", "sh", "-c", "echo $TEST"])
        .env("TEST", "VALUE")
        .assert()
        .success()
        .stdout("VALUE\n")
        .stderr("");
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
