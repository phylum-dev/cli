use std::ffi::OsStr;
#[cfg(feature = "extensions")]
use std::fs::File;
#[cfg(feature = "extensions")]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use assert_cmd::assert::Assert;
use assert_cmd::Command;
use phylum_cli::api::{PhylumApi, PhylumApiError, ResponseError};
use phylum_cli::config::{AuthInfo, Config, ConnectionInfo};
#[cfg(feature = "extensions")]
use phylum_cli::permissions::Permissions;
use phylum_types::types::auth::RefreshToken;
use reqwest::StatusCode;
use tempfile::TempDir;

pub const API_URL: &str = "https://api.staging.phylum.io";
const PROJECT_NAME: &str = "integration-tests";

#[derive(Default)]
enum Cwd {
    Path(PathBuf),
    TempDir,
    #[default]
    None,
}

#[derive(Default)]
pub struct TestCliBuilder {
    config: Option<Config>,
    cwd: Cwd,
}

impl TestCliBuilder {
    pub fn build(self) -> TestCli {
        let tempdir = TempDir::new().unwrap();
        let config_path = self.config.map(|config| create_config(tempdir.path(), config));

        let cwd = match self.cwd {
            Cwd::Path(p) => Some(p),
            Cwd::TempDir => Some(tempdir.path().to_owned()),
            Cwd::None => None,
        };

        TestCli { tempdir, cwd, config_path }
    }

    /// If true, a configuration will be generated, stored and passed as an
    /// option.
    pub fn with_config(mut self, config: impl Into<Option<Config>>) -> Self {
        self.config = Some(config.into().unwrap_or_else(|| {
            let mut config = Config::default();
            config.connection = ConnectionInfo { uri: API_URL.into() };
            config
        }));
        self
    }

    /// Set the current working directory of the CLI to the provided path.
    pub fn cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Cwd::Path(cwd);
        self
    }

    /// Set the current working directory of the CLI to the path of the
    /// temporary directory that is created.
    pub fn cwd_temp(mut self) -> Self {
        self.cwd = Cwd::TempDir;
        self
    }
}

pub struct TestCli {
    tempdir: TempDir,
    cwd: Option<PathBuf>,
    config_path: Option<PathBuf>,
}

impl Default for TestCli {
    fn default() -> Self {
        Self { tempdir: TempDir::new().unwrap(), cwd: None, config_path: None }
    }
}

impl TestCli {
    pub fn builder() -> TestCliBuilder {
        Default::default()
    }

    pub fn temp_path(&self) -> &Path {
        self.tempdir.path()
    }

    pub fn data_home(&self) -> PathBuf {
        self.temp_path().join("data")
    }

    pub fn state_home(&self) -> PathBuf {
        self.temp_path().join("state")
    }

    #[cfg(feature = "extensions")]
    pub fn install_extension(&self, path: &Path) -> Assert {
        self.run(["extension", "install", "-y", &path.to_string_lossy()])
    }

    #[cfg(feature = "extensions")]
    pub fn extension<'a>(&'a self, code: &'a str) -> TestExtensionBuilder<'a> {
        TestExtensionBuilder::new(self, code)
    }

    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("phylum").unwrap();

        cmd.env("XDG_DATA_HOME", self.data_home());
        cmd.env("XDG_STATE_HOME", self.state_home());

        if let Some(cwd) = self.cwd.as_ref() {
            cmd.current_dir(cwd);
        }

        if let Some(config_path) = self.config_path.as_ref() {
            cmd.arg("--config").arg(config_path);
        }

        cmd
    }

    pub fn run(&self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Assert {
        self.cmd().args(args).assert()
    }
}

#[cfg(feature = "extensions")]
pub struct TestExtensionBuilder<'a> {
    permissions: Permissions,
    test_cli: &'a TestCli,
    code: &'a str,
}

#[cfg(feature = "extensions")]
impl<'a> TestExtensionBuilder<'a> {
    fn new(test_cli: &'a TestCli, code: &'a str) -> Self {
        Self { test_cli, code, permissions: Default::default() }
    }

    pub fn build(&mut self) -> TestExtension<'a> {
        TestExtension::new(self.test_cli, self.code, &self.permissions)
    }

    pub fn with_permissions(&mut self, permissions: Permissions) -> &mut Self {
        self.permissions = permissions;
        self
    }
}

#[cfg(feature = "extensions")]
pub struct TestExtension<'a> {
    test_cli: &'a TestCli,
    extension_path: PathBuf,
}

#[cfg(feature = "extensions")]
impl<'a> TestExtension<'a> {
    fn new(test_cli: &'a TestCli, code: &str, permissions: &Permissions) -> Self {
        let extension_path = test_cli.temp_path().join("test-ext");

        // Create skeleton extension.
        test_cli.run(["extension", "new", &extension_path.to_string_lossy()]).success();

        // Overwrite skeleton code.
        let main = extension_path.join("main.ts");
        fs::write(main, code).unwrap();

        // Overwrite permissions.
        let manifest_path = extension_path.join("PhylumExt.toml");
        let mut manifest = File::options().append(true).open(manifest_path).unwrap();
        let permissions_str = toml::to_string(&permissions).unwrap();
        write!(manifest, "[permissions]\n{permissions_str}").unwrap();

        // Install extension.
        test_cli.run(["extension", "install", "-y", &extension_path.to_string_lossy()]);

        Self { test_cli, extension_path }
    }

    pub fn run(&self) -> Assert {
        // Execute extension.
        self.test_cli.run(["test-ext"])
    }
}

#[cfg(feature = "extensions")]
impl Drop for TestExtension<'_> {
    fn drop(&mut self) {
        self.test_cli.run(["extension", "uninstall", "test-ext"]).success();
        fs::remove_dir_all(&self.extension_path).unwrap();
    }
}

/// Create config file for the desired environment.
pub fn create_config(dir: &Path, config: Config) -> PathBuf {
    let config_path = dir.join("settings.yml");
    let config_yaml = serde_yaml::to_string(&config).expect("serialize config");
    fs::write(&config_path, config_yaml.as_bytes()).expect("writing config");

    config_path
}

/// Create a simple test lockfile.
pub fn create_lockfile(dir: &Path) -> PathBuf {
    let lockfile = dir.join("yarn.lock");
    fs::write(
        &lockfile,
        br#"
        __metadata:
          version: 6
          cacheKey: 8
        "accepts@npm:~1.3.8":
          version: 1.3.8
          resolution: "accepts@npm:1.3.8"
          checksum: 50c43d32e7b50285ebe84b613ee4a3aa426715a7d131b65b786e2ead0fd76b6b60091b9916d3478a75f11f162628a2139991b6c03ab3f1d9ab7c86075dc8eab4
          languageName: node
          linkType: hard
    "#,
    )
    .unwrap();
    lockfile
}

/// Ensure the specified project exists.
pub async fn create_project() -> &'static str {
    let offline_access = Some(RefreshToken::new(env::var("PHYLUM_API_KEY").unwrap()));
    let mut config = Config::default();
    config.connection = ConnectionInfo { uri: API_URL.into() };
    config.auth_info = AuthInfo::new(offline_access);

    // Attempt to create the project, ignoring conflicts.
    let api = PhylumApi::new(config, None).await.unwrap();
    match api.create_project(PROJECT_NAME, None, None, None).await {
        Ok(_) | Err(PhylumApiError::Response(ResponseError { code: StatusCode::CONFLICT, .. })) => {
        },
        err @ Err(_) => {
            err.unwrap();
        },
    }

    PROJECT_NAME
}
