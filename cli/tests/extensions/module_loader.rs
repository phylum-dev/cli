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
use super::*;

// The fixture for this test requires one local .ts file, one local .js file,
// and one file from Deno's standard library.
#[test]
fn good_module_loads_successfully() {
    let tempdir = TempDir::new().unwrap();

    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("extension")
        .arg("install")
        .arg(fixtures_path().join("module-import-extension").join("successful"))
        .assert()
        .success();

    let cmd = Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("module-import-success")
        .assert()
        .success();

    let stdout = std::str::from_utf8(&cmd.get_output().stdout).unwrap();
    assert!(stdout.contains("I should contain 12345"));
}

// The fixture for this test attempts a directory traversal.
#[test]
fn module_with_traversal_fails_to_load() {
    let tempdir = TempDir::new().unwrap();

    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("extension")
        .arg("install")
        .arg(fixtures_path().join("module-import-extension").join("fail-local"))
        .assert()
        .success();

    let cmd = Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("module-import-fail-local")
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&cmd.get_output().stderr).unwrap();
    assert!(stderr.contains("importing from paths outside"));
}

// The fixture for this test attempts to load a module from a non-`deno.land`
// URL.
#[test]
fn module_with_non_allowed_url_fails_to_load() {
    let tempdir = TempDir::new().unwrap();

    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("extension")
        .arg("install")
        .arg(fixtures_path().join("module-import-extension").join("fail-remote"))
        .assert()
        .success();

    let cmd = Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("module-import-fail-remote")
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&cmd.get_output().stderr).unwrap();
    assert!(stderr.contains("importing from domains other than"));
}

// A symlink is directly created during the test, as no symlinks are committed
// to the repo.
#[cfg(unix)]
#[test]
fn symlinks_are_rejected() {
    let tempdir = TempDir::new().unwrap();
    let ext_path = tempdir.path().join("phylum").join("extensions").join("symlink-extension");

    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .args(&["extension", "install"])
        .arg(fixtures_path().join("symlink-extension"))
        .assert()
        .success();

    std::os::unix::fs::symlink(ext_path.join("symlink_me.ts"), ext_path.join("symlink.ts"))
        .unwrap();

    Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir.path())
        .arg("symlink-extension")
        .assert()
        .failure()
        .stderr(predicate::str::contains("importing from symlinks is not allowed"));
}
