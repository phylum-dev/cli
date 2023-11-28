use std::process;

pub mod auth;
pub mod extensions;
pub mod find_lockable_files;
pub mod generate_lockfile;
pub mod group;
pub mod init;
pub mod jobs;
pub mod packages;
pub mod parse;
pub mod project;
#[cfg(unix)]
pub mod sandbox;
pub mod status;
#[cfg(feature = "selfmanage")]
pub mod uninstall;

/// Shorthand type for Result whose ok value is CommandValue
pub type CommandResult = anyhow::Result<ExitCode>;

/// Unique exit code values.
#[derive(Copy, Clone)]
pub enum ExitCode {
    Ok,
    Generic,
    NotAuthenticated,
    AuthenticationFailure,
    PackageNotFound,
    AlreadyExists,
    NoHistoryFound,
    JsError,
    ConfirmationFailed,
    NotFound,
    InvalidTokenExpiration,
    ManifestWithoutGeneration,
    FailedPolicy,
    SandboxStart,
    SandboxStartCollision,
    Custom(i32),
}

impl ExitCode {
    /// Terminate the application with this exit code.
    pub fn exit(&self) -> ! {
        process::exit(self.into());
    }
}

impl From<&ExitCode> for i32 {
    fn from(code: &ExitCode) -> Self {
        match code {
            ExitCode::Ok => 0,
            ExitCode::Generic => 1,
            ExitCode::NotAuthenticated => 10,
            ExitCode::AuthenticationFailure => 11,
            ExitCode::PackageNotFound => 12,
            ExitCode::AlreadyExists => 14,
            ExitCode::NoHistoryFound => 15,
            ExitCode::JsError => 16,
            ExitCode::ConfirmationFailed => 17,
            ExitCode::NotFound => 18,
            ExitCode::InvalidTokenExpiration => 19,
            ExitCode::ManifestWithoutGeneration => 20,
            ExitCode::FailedPolicy => 100,
            ExitCode::SandboxStart => 117,
            ExitCode::SandboxStartCollision => 118,
            ExitCode::Custom(code) => *code,
        }
    }
}
