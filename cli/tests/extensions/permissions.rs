use std::ffi::OsStr;

use assert_cmd::assert::Assert;
use predicates::prelude::*;

use super::*;

struct TestCli {
    tempdir: TempDir,
    cwd: Option<PathBuf>,
}

impl Default for TestCli {
    fn default() -> Self {
        Self { tempdir: TempDir::new().unwrap(), cwd: None }
    }
}

impl TestCli {
    fn new() -> Self {
        Default::default()
    }

    fn cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    fn install_extension(&self, path: &Path) -> Assert {
        self.run(&["extension", "install", "-y", &path.to_string_lossy()])
    }

    fn run<S: AsRef<str> + AsRef<OsStr>>(&self, args: &[S]) -> Assert {
        let mut cmd = Command::cargo_bin("phylum").unwrap();

        cmd.env("XDG_DATA_HOME", self.tempdir.path()).args(args);

        if let Some(cwd) = self.cwd.as_ref() {
            cmd.current_dir(cwd);
        }

        cmd.assert()
    }
}

#[test]
fn permission_dialog_is_shown_without_yes_flag() {
    let test_cli = TestCli::new().cwd(fixtures_path().join("permissions"));

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
    let test_cli = TestCli::new().cwd(fixtures_path().join("permissions"));

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
    let test_cli = TestCli::new().cwd(fixtures_path().join("permissions"));

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
    let test_cli = TestCli::new().cwd(fixtures_path().join("permissions"));

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-net-perms"))
        .success();

    test_cli.run(&["correct-net-perms"]).success().stdout(predicate::str::contains("upload_url"));
}

#[test]
fn incorrect_net_permission_unsuccessful_run() {
    let test_cli = TestCli::new().cwd(fixtures_path().join("permissions"));

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("incorrect-net-perms"))
        .success();

    test_cli
        .run(&["incorrect-net-perms"])
        .failure()
        .stderr(predicate::str::contains(r#"Error: Requires net access to "api.github.com""#));
}

#[test]
fn correct_run_permission_successful_install_and_run() {
    let test_cli = TestCli::new().cwd(fixtures_path().join("permissions"));

    test_cli
        .install_extension(&fixtures_path().join("permissions").join("correct-run-perms"))
        .success();

    test_cli
        .run(&["correct-run-perms"])
        .success()
        .stdout(predicate::str::contains("install").or(predicate::str::contains("API rate limit")));
}
