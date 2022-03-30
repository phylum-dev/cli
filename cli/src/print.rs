use std::io;
use std::io::Write;

use ansi_term::Color::{Blue, Cyan};
use clap::Command;
use serde::Serialize;

use crate::api::PhylumApiError;
use crate::filter::Filter;
use crate::summarize::Summarize;

#[macro_export]
macro_rules! print_user_success {
    ($($tts:tt)*) => {
        eprint!("✅ ");
        eprintln!($($tts)*);
    }
}

#[macro_export]
macro_rules! print_user_warning {
    ($($tts:tt)*) => {
        eprint!("⚠️  ");
        eprintln!($($tts)*);
    }
}

#[macro_export]
macro_rules! print_user_failure {
    ($($tts:tt)*) => {
        eprint!("❗ ");
        eprintln!($($tts)*);
    }
}

pub fn print_response<T>(
    resp: &Result<T, PhylumApiError>,
    pretty_print: bool,
    filter: Option<Filter>,
) where
    T: std::fmt::Debug + Serialize + Summarize,
{
    log::debug!("==> {:?}", resp);

    match resp {
        Ok(resp) => {
            if pretty_print {
                resp.summarize(filter);
            } else {
                // Use write! as a workaround to avoid https://github.com/rust-lang/rust/issues/46016
                //  when piping output to an external program
                let mut stdout = io::stdout();
                write!(
                    &mut stdout,
                    "{}",
                    serde_json::to_string_pretty(&resp).unwrap_or_else(|e| {
                        log::error!("Failed to serialize json response: {}", e);
                        "".to_string()
                    })
                )
                .unwrap_or_else(|e| log::debug!("Failed writing to stdout: {}", e));
            }
        }
        Err(err) => {
            print_user_failure!("Response error:\n{}", err);
        }
    }
}

/// Prints a verbose message informing the user that an update is available.
pub fn print_update_message() {
    eprintln!(
        "---------------- {} ----------------\n",
        Cyan.paint("Update Available")
    );
    eprintln!("A new version of the Phylum CLI is available. Run");
    eprintln!(
        "\n\t{}\n\nto update to the latest version!\n",
        Blue.paint("phylum update")
    );
    eprintln!("{:-^50}\n\n", "");
}

pub fn print_sc_help(app: &mut Command, subcommand: &str) {
    for sc in app.get_subcommands_mut() {
        if sc.get_name() == subcommand {
            let _ = sc.print_help();
            break;
        }
    }
    println!();
}
