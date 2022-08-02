// Test that the rules of the module loader are obeyed:
// - Both TypeScript and JavaScript are supported.
// - Files under $XDG_DATA_HOME/phylum/extensions may be imported.
// - Symlinks are not allowed.
// - Remote URLs under https://deno.land are supported -- i.e., the Deno's
//   standard library.
// - No other URLs are supported.
//   - We explicitly test that a https:// url which is not under `deno.land` is
//     rejected.
//   - We explicitly test that a directory traversal attempt is rejected.
//
// These tests are based on the fixtures under
// `fixtures/module-import-extension`.
use crate::common::*;
use crate::extensions::fixtures_path;

// The fixture for this test requires one local .ts file, one local .js file,
// and one file from Deno's standard library.
#[test]
fn good_module_loads_successfully() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("module-import").join("successful")).success();

    test_cli
        .run(&["module-import-success"])
        .success()
        .stdout(predicate::str::contains("I should contain 12345"));
}

// The fixture for this test attempts a directory traversal.
#[test]
fn module_with_traversal_fails_to_load() {
    let test_cli = TestCli::builder().build();

    test_cli.install_extension(&fixtures_path().join("module-import").join("successful")).success();
    test_cli.install_extension(&fixtures_path().join("module-import").join("fail-local")).success();

    test_cli
        .run(&["module-import-fail-local"])
        .failure()
        .stderr(predicate::str::contains("importing from paths outside"));
}

// The fixture for this test attempts to load a module from a non-`deno.land`
// URL.
#[test]
fn module_with_non_allowed_url_fails_to_load() {
    let test_cli = TestCli::builder().build();

    test_cli
        .install_extension(&fixtures_path().join("module-import").join("fail-remote"))
        .success();

    test_cli
        .run(&["module-import-fail-remote"])
        .failure()
        .stderr(predicate::str::contains("importing from domains other than"));
}
