use phylum_cli::permissions::{Permission, Permissions};
use predicates::prelude::*;

use crate::extensions::{fixtures_path, TestCli};

#[test]
fn permission_dialog_is_shown_without_yes_flag() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .run([
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
        .run(["correct-read-perms"])
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
        .run(["incorrect-read-perms"])
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
        .run(["correct-net-perms"])
        .success()
        .stdout(predicate::str::contains("<!DOCTYPE html>"));
}

#[test]
fn incorrect_net_permission_unsuccessful_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("incorrect-net-perms"))
        .success();

    test_cli
        .run(["incorrect-net-perms"])
        .failure()
        .stderr("❗ Error: Requires net access to \"phylum.io\"\n");
}

#[test]
#[cfg(unix)]
fn correct_sandbox_run_permission_successful_install_and_run() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-run-perms"))
        .success();

    test_cli.run(["correct-run-perms"]).success().stdout(predicate::str::contains("hello"));
}

#[test]
#[cfg(not(unix))]
fn correct_sandbox_run_permission_fail_on_windows() {
    let test_cli = TestCli::builder().cwd(fixtures_path().join("permissions")).build();

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-run-perms"))
        .success();

    test_cli
        .run(&["correct-run-perms"])
        .failure()
        .stderr(predicate::str::contains("Extension sandboxing is not supported on this platform"));
}

#[test]
fn incorrect_run_permission() {
    let test_cli = TestCli::builder().build();

    #[rustfmt::skip]
    test_cli
        .extension("
            const cmd = new Deno.Command('echo', { args: ['hello'] });
            const output = await cmd.spawn().status;
            Deno.exit(output.code);
        ")
        .with_permissions(Permissions {
            unsandboxed_run: Permission::Boolean(false),
            ..Permissions::default()
        })
        .build()
        .run()
        .failure()
        .stderr("❗ Error: Requires run access to \"echo\"\n");
}

#[test]
fn correct_run_permission() {
    let test_cli = TestCli::builder().build();

    #[rustfmt::skip]
    test_cli
        .extension("
            const cmd = new Deno.Command('echo', { args: ['hello'] });
            const output = await cmd.spawn().status;
            Deno.exit(output.code);
        ")
        .with_permissions(Permissions {
            unsandboxed_run: Permission::List(vec!["echo".into()]),
            ..Permissions::default()
        })
        .build()
        .run()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[tokio::test]
pub async fn disallow_permission_request() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("await Deno.permissions.request({ name: 'net' });")
        .build()
        .run()
        .failure()
        .stderr(predicate::str::contains("Error: op is disabled"));
}

#[test]
fn permissions_op() {
    let test_cli = TestCli::builder().with_config(None).build();

    let permissions =
        Permissions { read: Permission::List(vec!["/tmp".to_string()]), ..Permissions::default() };

    let permissions_ext = "
         const perms = Phylum.permissions()
         console.log(perms);";

    test_cli
        .extension(permissions_ext)
        .with_permissions(permissions)
        .build()
        .run()
        .success()
        .stdout(predicate::str::contains(r#"read: [ "/tmp" ]"#));
}
