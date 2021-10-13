use phylum_cli::types::Action;

pub mod auth;
pub mod jobs;
pub mod lock_files;
pub mod packages;
pub mod projects;

/// The possible result values of commands
pub enum CommandValue {
    /// Do nothing
    Void,
    /// A response to print to the user
    String(String),
    /// An action to be undertaken wrt the build
    Action(Action),
}

impl From<Action> for CommandValue {
    fn from(action: Action) -> Self {
        Self::Action(action)
    }
}

impl From<&'static str> for CommandValue {
    fn from(str: &'static str) -> Self {
        Self::String(str.to_owned())
    }
}

/// Shorthand type for Result whose ok value is CommandValue
pub type CommandResult = anyhow::Result<CommandValue>;

impl From<CommandValue> for CommandResult {
    fn from(value: CommandValue) -> Self {
        Ok(value)
    }
}
