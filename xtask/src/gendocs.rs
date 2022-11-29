//! Convert Phylum CLI to markdown.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap_markdown::Generator;
use phylum_cli::app;
use phylum_cli::commands::extensions;

/// Output directory.
const OUTPUT_DIR: &str = "./docs/command_line_tool";

/// Template directory.
const TEMPLATE_DIR: &str = "./doc_templates";

/// File header inserted at the top of each page.
const HEADER: &str = "---
title: {PH-TITLE}
category: 6255e67693d5200013b1fa3e
hidden: false
---\n\n";

/// Generate Phylum CLI documentation.
pub fn gendocs() -> Result<()> {
    // Create target directory.
    fs::create_dir_all(Path::new(OUTPUT_DIR))?;

    // Store all generated markdown pages in target directory.
    for (path, markdown) in pages()? {
        fs::write(path, markdown)?;
    }

    Ok(())
}

/// Get a vec with all pages and their respective paths.
fn pages() -> Result<Vec<(PathBuf, String)>> {
    let template_dir = Path::new(TEMPLATE_DIR);
    let target_dir = Path::new(OUTPUT_DIR);

    // Find all installed extensions.
    let extensions = extensions::installed_extensions()?;

    // Load default template.
    let default_template = fs::read_to_string(template_dir.join("default.md"))
        .expect("missing default.md template");

    // Setup Markdown generator.
    let mut cli = app::app();
    let generator = Generator::new(&mut cli);

    let mut pages = Vec::new();

    for page in generator.generate() {
        // Skip documentation for extensions.
        if page.command.len() == 2 && extensions.iter().any(|ext| ext.name() == page.command[1]) {
            continue;
        }

        let file_name = format!("{}.md", page.command.join("_"));

        // Load markdown template.
        let mut markdown = fs::read_to_string(template_dir.join(&file_name))
            .unwrap_or_else(|_| default_template.clone());

        // Replace template placeholders.
        markdown = markdown.replace("{PH-HEADER}", HEADER);
        markdown = markdown.replace("{PH-TITLE}", &page.command.join(" "));
        markdown = markdown.replace("{PH-MARKDOWN}", &page.content);

        pages.push((target_dir.join(file_name), markdown));
    }

    Ok(pages)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    /// Ensure the generate CLI docs are always up-to-date.
    #[test]
    fn docs_up_to_date() {
        // Move to project root.
        env::set_current_dir("..").unwrap();

        // Ensure all pages are up-to-date.
        for (path, expected) in pages().unwrap() {
            let current = fs::read_to_string(&path).expect(&format!("missing {path:?} CLI docs"));
            assert_eq!(current, expected, "out of date {path:?} CLI docs");
        }
    }
}
