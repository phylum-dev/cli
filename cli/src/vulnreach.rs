//! Vulnerability reachability analysis.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;

use ignore::Walk;
use vuln_reach::javascript::lang::imports::{CommonJsImports, EsmImports};
use vuln_reach::Tree;

/// Get names of all imported packages.
pub fn imports() -> Vec<String> {
    let mut packages = HashSet::new();

    for entry in Walk::new(".")
        .flatten()
        .filter(|entry| entry.path().extension().and_then(OsStr::to_str) == Some("js"))
    {
        // Read in the JS source.
        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(_) => continue,
        };

        // Parse into syntax tree.
        let tree = match Tree::new(content) {
            Ok(tree) => tree,
            Err(_) => continue,
        };

        // Add all commonjs import packages.
        for import in CommonJsImports::try_from(&tree).iter().flatten() {
            if let Some(name) = truncate_imports(tree.repr_of(import.node())) {
                packages.insert(name);
            }
        }

        // Add all esm import packages.
        for import in EsmImports::try_from(&tree).iter().flatten() {
            if let Some(name) = truncate_imports(import.source()) {
                packages.insert(name);
            }
        }
    }

    packages.drain().collect()
}

/// Truncate imports to exclude their subpaths.
///
/// @angular/http/core => @angular/http
fn truncate_imports(import: &str) -> Option<String> {
    // Filter path imports.
    if import.starts_with('.') || import.starts_with('/') {
        return None;
    }

    // Check number of allowed path separators in the package name.
    let allowed_slashes = if import.starts_with('@') { 1 } else { 0 };

    // Truncate everything beyond the first unwanted slash.
    Some(import.split('/').take(allowed_slashes + 1).collect::<Vec<_>>().join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate() {
        // Paths imports.
        assert_eq!(truncate_imports("./modules"), None);
        assert_eq!(truncate_imports("../modules"), None);
        assert_eq!(truncate_imports("/test"), None);

        // Untruncated imports.
        assert_eq!(truncate_imports("core-js"), Some("core-js".to_string()));
        assert_eq!(truncate_imports("@angular/http"), Some("@angular/http".to_string()));

        // Truncated imports.
        assert_eq!(truncate_imports("core-js/compat"), Some("core-js".to_string()));
        assert_eq!(truncate_imports("@angular/http/core"), Some("@angular/http".to_string()));
        assert_eq!(truncate_imports("core-js/compat/deep/stuff"), Some("core-js".to_string()));
        assert_eq!(
            truncate_imports("@angular/http/core/deep/stuff"),
            Some("@angular/http".to_string())
        );
    }
}
