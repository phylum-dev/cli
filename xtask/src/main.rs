use std::env;
use std::fs::File;
use std::io::Write;
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
        Some("dist") => dist::dist(),
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

fn print_help() {}

//
// Dist task
//

mod dist {
    use super::*;

    pub(super) fn dist() -> Result<()> {
        let dist_root = project_root().join("target").join("dist");
        std::fs::remove_dir_all(&dist_root).ok();

        info!("Building Linux Docker image");
        docker_build_linux()?;
        info!("Building MacOS Docker image");
        docker_build_macos()?;

        dist_for_arch(
            &dist_root,
            "phylum-cli-build-linux",
            "target/x86_64-unknown-linux-musl/release/phylum",
            "linux-x86_64",
        )?;
        dist_for_arch(
            &dist_root,
            "phylum-cli-build-macos",
            "target/x86_64-apple-darwin/release/phylum",
            "macos-x86_64",
        )?;
        dist_for_arch(
            &dist_root,
            "phylum-cli-build-macos",
            "target/aarch64-apple-darwin/release/phylum",
            "macos-aarch64",
        )?;

        Ok(())
    }

    fn dist_for_arch(dist_root: &Path, image: &str, path: &str, arch: &str) -> Result<()> {
        info!("Building {arch} distribution");
        info!("  Copying phylum-{arch}");
        let project_root = project_root();
        let dist_root = dist_root.join(format!("phylum-{arch}"));
        let executable = dist_root.join("phylum");

        std::fs::create_dir_all(&dist_root)?;
        let macos_arm_bin = docker_get_bin(image, path)?;
        File::create(&executable)?.write_all(&macos_arm_bin)?;
        chmod_executable(&executable)?;

        info!("  Copying completions");
        copy_completions(&dist_root)?;

        info!("  Copying settings.yaml");
        std::fs::copy(
            project_root.join("cli").join("src").join("settings.yaml"),
            dist_root.join("settings.yaml"),
        )?;
        info!("  Copying install.sh");
        std::fs::copy(
            project_root.join("cli").join("src").join("install.sh"),
            dist_root.join("install.sh"),
        )?;

        Ok(())
    }

    fn docker_build_linux() -> Result<()> {
        let project_root = project_root();

        let status = Command::new("docker")
            .current_dir(&project_root)
            .args(&[
                "build",
                "-t",
                "phylum-cli-build-linux",
                "-f",
                "xtask/src/dockerfiles/Dockerfile.linux",
                &project_root.to_string_lossy(),
            ])
            .status()?;

        if !status.success() {
            Err(Error::msg("couldn't build linux docker image"))
        } else {
            Ok(())
        }
    }

    fn docker_build_macos() -> Result<()> {
        let project_root = project_root();

        let status = Command::new("docker")
            .current_dir(&project_root)
            .args(&[
                "build",
                "-t",
                "phylum-cli-build-macos",
                "-f",
                "xtask/src/dockerfiles/Dockerfile.macos",
                &project_root.to_string_lossy(),
            ])
            .status()?;

        if !status.success() {
            Err(Error::msg("couldn't build macos docker image"))
        } else {
            Ok(())
        }
    }

    fn docker_get_bin(image: &str, path: &str) -> Result<Vec<u8>> {
        let project_root = project_root();

        Ok(Command::new("docker")
            .current_dir(&project_root)
            .args(&[
                "run",
                "--rm",
                image,
                "cat",
                path,
                &project_root.to_string_lossy(),
            ])
            .output()?
            .stdout)
    }

    fn copy_completions(dest: &Path) -> Result<()> {
        let completions_path = dest.join("completions");
        let mut app = phylum_cli::app::app();

        std::fs::create_dir_all(&completions_path)?;

        info!("  Generating Bash completions");
        generate_to(Bash, &mut app, "phylum", &completions_path)?;
        info!("  Generating Zsh completions");
        generate_to(Zsh, &mut app, "phylum", &completions_path)?;
        info!("  Generating Fish completions");
        generate_to(Fish, &mut app, "phylum", &completions_path)?;

        Ok(())
    }

    fn chmod_executable(file: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            info!("Adjusting permissions: {}", file.to_string_lossy());
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(file)?.permissions();
            perm.set_mode(perm.mode() | 0o111);
            std::fs::set_permissions(file, perm)?;
        }

        Ok(())
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

fn install_root() -> Result<PathBuf> {
    home::home_dir()
        .ok_or_else(|| Error::msg("could not find home directory"))
        .map(|dir| dir.join(".phylum"))
}
