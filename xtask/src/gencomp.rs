//! Generate shell completion files.

use std::path::Path;

use anyhow::Result;
use clap_complete::generate_to;
use clap_complete::shells::{Bash, Fish, Zsh};
use log::info;

use crate::project_root;

/// Generate shell completion files.
pub fn gencomp() -> Result<()> {
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

#[cfg(test)]
mod test {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn generate_completions() {
        let tmp_dir = tempdir().unwrap();

        copy_completions(tmp_dir.path()).unwrap();
    }
}
