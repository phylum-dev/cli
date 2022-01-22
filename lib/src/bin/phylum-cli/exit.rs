use std::process;

use log::*;

use crate::print_user_warning;
use crate::{print_user_failure, print_user_success};

/// Exit with status code 0 and optionally print a message to the user.
pub fn exit_ok(message: Option<impl AsRef<str>>) -> ! {
    if let Some(message) = message {
        info!("{}", message.as_ref());
        print_user_success!("{}", message.as_ref());
    }
    process::exit(0)
}

/// Print a warning message to the user before exiting with exit code 0.
pub fn exit_warn(message: impl AsRef<str>) -> ! {
    warn!("{}", message.as_ref());
    print_user_warning!("Warning: {}", message.as_ref());
    process::exit(0)
}

/// Print an error to the user before exiting with exit code 1.
pub fn exit_fail(message: impl AsRef<str>) -> ! {
    error!("{}", message.as_ref());
    print_user_failure!("Error: {}", message.as_ref());
    process::exit(1)
}

/// Exit with status code 1, and optionally print a message to the user and
/// print error information.
pub fn exit_error(error: Box<dyn std::error::Error>, message: Option<impl AsRef<str>>) -> ! {
    match message {
        None => {
            error!("{}: {:?}", error, error);
            print_user_failure!("Error: {}", error);
        }
        Some(message) => {
            error!("{}: {:?}", message.as_ref(), error);
            print_user_failure!("Error: {} caused by: {}", message.as_ref(), error);
        }
    }
    process::exit(1)
}
