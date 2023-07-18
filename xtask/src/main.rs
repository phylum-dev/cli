use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Command;
use log::LevelFilter;
use phylum_cli::app;
use simplelog::{ColorChoice, TermLogger, TerminalMode};

mod gencomp;
mod gendocs;

fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        Default::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;
    match std::env::args().nth(1).as_deref() {
        Some("gendocs") => gendocs::gendocs(),
        Some("gencomp") => gencomp::gencomp(),
        None | Some("help") => {
            print_help();
            Ok(())
        },
        _ => {
            print_help();
            Ok(())
        },
    }
}

/// Print command usage.
fn print_help() {
    println!(
        r#"
    Usage

    cargo run -p xtask <task>

    Available tasks:

    gendocs ....... Generate CLI documentation files
    gencomp ....... Generate completion files
    "#
    );
}

/// Return the repository root directory.
fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
}

/// Get the CLI app without extensions.
fn cli_app() -> Command {
    // Set `XDG_DATA_HOME` to a bogus directory so regardless of installed
    // extensions, none of them are ever documented.
    env::set_var("XDG_DATA_HOME", "/i/n/v/a/l/i/d");

    app::app()
}
