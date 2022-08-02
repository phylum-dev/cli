use std::process;

use phylum_types::types::job::Action;

pub mod auth;
#[cfg(feature = "extensions")]
pub mod extensions;
pub mod group;
pub mod jobs;
pub mod packages;
pub mod parse;
pub mod project;
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
    Ok = 0,
    Generic = 1,
    NotAuthenticated = 10,
    AuthenticationFailure = 11,
    PackageNotFound = 12,
    SetThresholdsFailure = 13,
    AlreadyExists = 14,
    NoHistoryFound = 15,
    JsError = 16,
    FailedThresholds = 100,
}

impl ExitCode {
    /// Terminate the application with this exit code.
    pub fn exit(&self) -> ! {
        process::exit(*self as i32);
    }
}
