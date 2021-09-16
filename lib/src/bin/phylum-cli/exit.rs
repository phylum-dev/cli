use std::process;

use crate::print_user_failure;
use crate::print_user_warning;
use std::error::Error;

pub fn err_exit(error: impl Error, message: &str, code: i32) -> ! {
    log::error!("{}: {:?}", message, error);
    print_user_failure!("Error: {}", message);
    process::exit(code);
}

pub fn exit(message: Option<&str>, code: i32) -> ! {
    if let Some(message) = message {
        if code != 0 {
            log::warn!("{}", message);
            print_user_failure!("Error: {}", message);
        } else {
            log::debug!("{}", message);
            print_user_warning!("{}", message);
        }
    }
    process::exit(code);
}
