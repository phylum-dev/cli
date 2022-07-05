use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub use assert_cmd::assert::Assert;
pub use assert_cmd::Command;
pub use predicates::prelude::*;
use tempfile::TempDir;

pub struct TestCli {
    tempdir: TempDir,
    cwd: Option<PathBuf>,
}

#[derive(Default)]
pub struct TestCliBuilder {
    cwd: Option<PathBuf>,
}

impl TestCliBuilder {
    pub fn build(self) -> TestCli {
        TestCli { tempdir: TempDir::new().unwrap(), cwd: self.cwd }
    }

    pub fn cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }
}

impl Default for TestCli {
    fn default() -> Self {
        Self { tempdir: TempDir::new().unwrap(), cwd: None }
    }
}

impl TestCli {
    pub fn builder() -> TestCliBuilder {
        Default::default()
    }

    pub fn temp_path(&self) -> &Path {
        self.tempdir.path()
    }

    pub fn install_extension(&self, path: &Path) -> Assert {
        self.run(&["extension", "install", "-y", &path.to_string_lossy()])
    }

    pub fn run<S: AsRef<str> + AsRef<OsStr>>(&self, args: &[S]) -> Assert {
        let mut cmd = Command::cargo_bin("phylum").unwrap();

        cmd.env("XDG_DATA_HOME", self.tempdir.path()).args(args);

        if let Some(cwd) = self.cwd.as_ref() {
            cmd.current_dir(cwd);
        }

        cmd.assert()
    }
}
