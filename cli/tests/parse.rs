use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use crate::common::TestCli;

#[test]
fn parse_with_project_lockfile() {
    // Setup CLI with temp dir.
    let test_cli = TestCli::builder().cwd_temp().build();
    let temp_path = test_cli.temp_path();

    // Write .phylum_project to temp dir.
    let config = "id: 00000000-0000-0000-0000-000000000000\nname: test\ncreated_at: \
                  2000-01-01T00:00:00.0Z\nlockfile_path: ./package-lock.json\nlockfile_type: npm";
    fs::write(temp_path.join(".phylum_project"), config).unwrap();

    // Copy lockfile to temp dir.
    fs::copy("../tests/fixtures/package-lock.json", temp_path.join("package-lock.json")).unwrap();

    test_cli
        .cmd()
        .args(["parse"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"typescript\""));
}

#[test]
fn parse_with_project_lockfile_relative_paths() {
    // Setup CLI with temp dir.
    let tempdir = TempDir::new().unwrap();
    let sensitive_dir = tempdir.path().join("sensitive_dir_name");
    fs::create_dir_all(&sensitive_dir).unwrap();
    let test_cli = TestCli::builder().cwd(sensitive_dir.clone()).build();

    // Write .phylum_project to temp dir.
    let config = "id: 00000000-0000-0000-0000-000000000000\nname: test\ncreated_at: \
                  2000-01-01T00:00:00.0Z\nlockfile_path: ./package-lock.json\nlockfile_type: npm";
    fs::write(sensitive_dir.join(".phylum_project"), config).unwrap();

    // Copy lockfile to temp dir.
    fs::copy("../tests/fixtures/package-lock.json", sensitive_dir.join("./package-lock.json"))
        .unwrap();

    let not_sensitive_dir = predicate::str::contains("sensitive_dir_name").not();
    test_cli.cmd().args(["parse"]).assert().success().stdout(not_sensitive_dir);
}

#[test]
fn parse_nonstandard_pip_manifest() {
    // Setup CLI with temp dir.
    let test_cli = TestCli::builder().cwd_temp().build();
    let temp_path = test_cli.temp_path();

    // Copy non-standard named pip manifest file to temp dir.
    fs::copy("../tests/fixtures/dev-requirements.txt", temp_path.join("dev-requirements.txt"))
        .unwrap();

    test_cli.cmd().args(["parse", "--type", "pip", "dev-requirements.txt"]).assert().success();
}
