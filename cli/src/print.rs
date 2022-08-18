use std::borrow::Cow;

use ansi_term::Color::{Blue, Cyan};
use clap::Command;
use prettytable::format;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[macro_export]
macro_rules! print_user_success {
    ($($tts:tt)*) => {{
        eprint!("✅ ");
        eprintln!($($tts)*);
    }}
}

#[macro_export]
macro_rules! print_user_warning {
    ($($tts:tt)*) => {{
        eprint!("⚠️  ");
        eprintln!($($tts)*);
    }}
}

#[macro_export]
macro_rules! print_user_failure {
    ($($tts:tt)*) => {{
        eprint!("❗ ");
        eprintln!($($tts)*);
    }}
}

/// Prints a verbose message informing the user that an update is available.
pub fn print_update_message() {
    eprintln!("---------------- {} ----------------\n", Cyan.paint("Update Available"));
    eprintln!("A new version of the Phylum CLI is available. Run");
    eprintln!("\n\t{}\n\nto update to the latest version!\n", Blue.paint("phylum update"));
    eprintln!("{:-^50}\n\n", "");
}

pub fn print_sc_help(mut app: &mut Command, subcommands: &[&str]) {
    for subcommand in subcommands {
        match app.get_subcommands_mut().find(|sc| &sc.get_name() == subcommand) {
            Some(subcommand) => app = subcommand,
            // Subcommand doesn't exist; don't print anything.
            None => return,
        }
    }

    let _ = app.print_help();
}

/// Limit a string to a specific length, using an ellipsis to indicate
/// truncation.
pub fn truncate(text: &str, max_length: usize) -> Cow<str> {
    if text.width() > max_length {
        let mut len = 0;
        let truncated = text
            .chars()
            .take_while(|c| {
                len += c.width().unwrap_or(0);
                len < max_length
            })
            .collect::<String>()
            .trim_end()
            .to_owned()
            + "…";
        Cow::Owned(truncated)
    } else {
        Cow::Borrowed(text)
    }
}

pub fn table_format(left_pad: usize, right_pad: usize) -> format::TableFormat {
    format::FormatBuilder::new()
        .column_separator(' ')
        .borders(' ')
        .separators(
            &[format::LinePosition::Top, format::LinePosition::Bottom],
            format::LineSeparator::new(' ', ' ', ' ', ' '),
        )
        .padding(left_pad, right_pad)
        .build()
}
