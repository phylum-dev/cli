use test_utils::*;

use crate::*;

#[test]
fn permission_dialog_is_shown_without_yes_flag() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .run(&[
            "extension",
            "install",
            &fixtures_path().join("permissions").join("correct-read-perms").to_string_lossy(),
        ])
        .failure()
        .stderr(predicate::str::contains("Can't ask for permissions"));
}

#[test]
fn correct_read_permission_successful_install_and_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-read-perms"))
        .success();

    test_cli
        .run(&["correct-read-perms"])
        .success()
        .stdout(predicate::str::contains("await Deno.readFile"));
}

#[test]
fn incorrect_read_permission_unsuccessful_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("incorrect-read-perms"))
        .success();

    test_cli
        .run(&["incorrect-read-perms"])
        .failure()
        .stderr(predicate::str::contains("Error: Requires read access"));
}

#[test]
fn correct_net_permission_successful_install_and_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-net-perms"))
        .success();

    test_cli
        .run(&["correct-net-perms"])
        .success()
        .stdout(predicate::str::contains("<!doctype html>"));
}

#[test]
fn incorrect_net_permission_unsuccessful_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("incorrect-net-perms"))
        .success();

    test_cli
        .run(&["incorrect-net-perms"])
        .failure()
        .stderr(predicate::str::contains(r#"Error: Requires net access to "phylum.io""#));
}

#[test]
fn correct_run_permission_successful_install_and_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-run-perms"))
        .success();

    test_cli.run(&["correct-run-perms"]).success().stdout(predicate::str::contains("install"));
}
