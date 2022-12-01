use std::process;

use phylum_types::types::job::Action;

pub mod auth;
pub mod extensions;
pub mod group;
pub mod init;
pub mod jobs;
pub mod packages;
pub mod parse;
pub mod project;
#[cfg(unix)]
pub mod sandbox;
#[cfg(feature = "selfmanage")]
pub mod uninstall;

/// The possible result values of commands
pub enum CommandValue {
    /// Exit with a specific code.
    Code(ExitCode),
    /// An action to be undertaken wrt the build
    Action(Action),
}

impl From<ExitCode> for CommandValue {
    fn from(code: ExitCode) -> Self {
        Self::Code(code)
    }
}

/// Shorthand type for Result whose ok value is CommandValue
pub type CommandResult = anyhow::Result<CommandValue>;

/// Unique exit code values.
#[derive(Copy, Clone)]
pub enum ExitCode {
    Ok,
    Generic,
    NotAuthenticated,
    AuthenticationFailure,
    PackageNotFound,
    SetThresholdsFailure,
    AlreadyExists,
    NoHistoryFound,
    JsError,
    FailedThresholds,
    ConfirmationFailed,
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
            ExitCode::SetThresholdsFailure => 13,
            ExitCode::AlreadyExists => 14,
            ExitCode::NoHistoryFound => 15,
            ExitCode::JsError => 16,
            ExitCode::ConfirmationFailed => 17,
            ExitCode::FailedThresholds => 100,
            ExitCode::Custom(code) => *code,
        }
    }
}
