use crate::common::*;
use crate::extensions::fixtures_path;

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
        .stderr("❗ Error: Requires read access to \"/tmp/passwd\"\n");
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
        .stderr("❗ Error: Requires net access to \"phylum.io\"\n");
}

#[test]
fn deno_run_api_disabled() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .extension(
            r#"
        try {
            await Deno.run("/bin/echo")
        } catch(e) {
            console.log("OK")
        }
    "#,
        )
        .build()
        .run()
        .success()
        .stdout(predicate::str::contains("OK"));
}

#[tokio::test]
pub async fn get_package_details() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension(
            "\
        await Deno.permissions.request({ name: 'net' });
        await fetch('https://phylum.io');\
             ",
        )
        .build()
        .run()
        .failure()
        .stderr("❗ Error: Requires net access to \"phylum.io\"\n");
}
