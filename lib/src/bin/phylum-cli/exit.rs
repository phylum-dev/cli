use std::error::Error;
use std::process;

use clap::App;
use log::*;

use crate::print::print_sc_help;
use crate::print_user_warning;
use crate::{print_user_failure, print_user_success};

/// Print help infomation for the command and exit with status code 0.
pub fn print_help_exit(app: &mut App, subcommand: &str) -> ! {
    print_sc_help(app, subcommand);
    process::exit(0)
}

/// Exit with status code 0 and optionally print a message to the user.
pub fn exit_ok(message: Option<impl AsRef<str>>) -> ! {
    message.map(|message| {
        warn!("{}", message.as_ref());
        print_user_success!("{}", message.as_ref());
    });
    process::exit(0)
}

/// Print a warning message to the user before exiting with exit code 0.
pub fn exit_warn(message: impl AsRef<str>) -> ! {
    warn!("{}", message.as_ref());
    print_user_warning!("Warning: {}", message.as_ref());
    process::exit(0)
}

pub fn exit_fail(message: impl AsRef<str>) -> ! {
    error!("{}", message.as_ref());
    print_user_failure!("Error: {}", message.as_ref());
    process::exit(1)
}

/// Exit with status code 1, and optionally print a message to the user and
/// print error information.
pub fn exit_error(error: impl Error, message: Option<impl AsRef<str>>) -> ! {
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
