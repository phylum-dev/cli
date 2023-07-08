//! Convert Phylum CLI to markdown.

use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::Result;
use clap_markdown::Generator;
use phylum_cli::app;

/// Output directory.
const OUTPUT_DIR: &str = "./docs/commands";

/// Template directory.
const TEMPLATE_DIR: &str = "./doc_templates";

/// File header inserted at the top of each page.
const HEADER: &str = "---
title: {PH-TITLE}
---";

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

    // Set `XDG_DATA_HOME` to a bogus directory so regardless of installed
    // extensions, none of them are ever documented.
    env::set_var("XDG_DATA_HOME", "/i/n/v/a/l/i/d");

    // Load default template.
    let default_template =
        fs::read_to_string(template_dir.join("default.md")).expect("missing default.md template");

    // Setup Markdown generator.
    let mut cli = app::app();
    let generator = Generator::new(&mut cli);

    let mut pages = Vec::new();

    for page in generator.generate() {
        let file_name = format!("{}.md", page.command.join("_"));

        // Load markdown template.
        let mut markdown = fs::read_to_string(template_dir.join(&file_name))
            .unwrap_or_else(|_| default_template.clone());

        // Remove trailing newline from markdown.
        let content = page.content.strip_suffix('\n').unwrap_or(&page.content);

        // Replace template placeholders.
        markdown = markdown.replace("{PH-HEADER}", HEADER);
        markdown = markdown.replace("{PH-TITLE}", &page.command.join(" "));
        markdown = markdown.replace("{PH-MARKDOWN}", content);

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
            let current =
                fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {path:?} CLI docs"));
            assert_eq!(current, expected, "out of date {path:?} CLI docs");
        }
    }
}
