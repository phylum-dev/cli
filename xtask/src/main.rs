use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Error, Result};
use clap_complete::{generate_to, shells::*};
use log::*;
use simplelog::ColorChoice;
use simplelog::TerminalMode;

fn main() -> Result<()> {
    simplelog::TermLogger::init(
        LevelFilter::Info,
        Default::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;
    match std::env::args().nth(1).as_deref() {
        Some("gencomp") => gencomp::gencomp(),
        Some("test") => cli_args_test::test(),
        None | Some("help") => {
            print_help();
            Ok(())
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

//
// Help task
//

fn print_help() {
    println!(
        r#"
    Usage

    cargo xtask <task>

    Available tasks:

    gencomp ....... Generate completion files
    test .......... Run various CLI subcommand paths
    "#
    );
}

//
// generate-completions task
//

mod gencomp {
    use super::*;

    pub(super) fn gencomp() -> Result<()> {
        let comp_root = project_root().join("target").join("completions");
        let _ = std::fs::remove_dir_all(&comp_root);

        copy_completions(&comp_root)?;

        Ok(())
    }

    fn copy_completions(completions_path: &Path) -> Result<()> {
        let mut app = phylum_cli::app::app();

        std::fs::create_dir_all(completions_path)?;

        info!("  Generating Bash completions");
        generate_to(Bash, &mut app, "phylum", completions_path)?;
        info!("  Generating Zsh completions");
        generate_to(Zsh, &mut app, "phylum", completions_path)?;
        info!("  Generating Fish completions");
        generate_to(Fish, &mut app, "phylum", completions_path)?;

        Ok(())
    }
}

//
// CLI arguments test
//

mod cli_args_test {
    use super::*;

    pub(super) fn test() -> Result<()> {
        copy_fixtures()?;

        let tests = [
            vec!["auth", "status"],
            vec!["history"],
            vec!["package", "react", "16.13.0"],
            vec!["ping"],
            vec!["project"],
            vec!["project", "create", "test-project"],
            vec!["project", "link", "test-project"],
            vec!["project", "--json"],
            vec!["project", "--json", "list"],
            vec!["project", "list", "--json"],
            vec!["analyze", "yarn.lock"],
            vec!["analyze", "yarn.lock", "--json"],
            vec!["version"],
        ]
        .into_iter()
        .map(|args| (args.clone(), run_cli_with_args(&args)))
        .collect::<Vec<_>>();

        println!("\nTest report\n");

        for (args, outcome) in tests {
            match outcome {
                Ok(()) => println!("✅ phylum {}", args.join(" ")),
                Err(e) => println!("❌ phylum {}: {}", args.join(" "), e),
            }
        }

        Ok(())
    }

    fn copy_fixtures() -> Result<()> {
        let src = project_root()
            .join("xtask")
            .join("fixtures")
            .join("test-project");
        let dst = project_root().join("target").join("tmp");
        std::fs::remove_dir_all(&dst).ok();
        std::fs::create_dir_all(&dst).ok();
        fs_extra::dir::copy(&src, &dst, &fs_extra::dir::CopyOptions::new())?;
        Ok(())
    }

    fn run_cli_with_args(phylum_args: &[&str]) -> Result<()> {
        print!("\n  🔎 Running `phylum");
        for a in phylum_args {
            print!(" {a}");
        }
        println!("`\n");

        let workdir = project_root()
            .join("target")
            .join("tmp")
            .join("test-project");
        let mut args = vec!["run", "--quiet", "--bin", "phylum", "--"];
        args.extend(phylum_args);
        let status = Command::new("cargo")
            .current_dir(&workdir)
            .args(&args)
            .status()?;

        if !status.success() {
            Err(Error::msg("cargo run failed"))
        } else {
            Ok(())
        }
    }
}

//
// Utilities
//

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
