use std::fs;

use predicates::prelude::*;

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
        .args(&["parse"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"typescript\""));
}
