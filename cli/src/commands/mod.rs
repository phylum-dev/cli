use phylum_types::types::job::Action;

pub mod auth;
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
pub enum ExitCode {
    Ok = 0,
    NotAuthenticated = 10,
    AuthenticationFailure = 11,
    PackageNotFound = 12,
    SetThresholdsFailure = 13,
}
