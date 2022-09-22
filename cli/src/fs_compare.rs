//! Filesystem comparison utilities

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

/// Compare the contents of two directories
pub fn dir_compare<A: AsRef<Path>, B: AsRef<Path>>(a: A, b: B) -> Result<bool> {
    let a = WalkDir::new(a).sort_by_file_name();
    let b = WalkDir::new(b).sort_by_file_name();

    for (a, b) in a.into_iter().zip(b) {
        let a = a?;
        let b = b?;

        if a.depth() == 0 && b.depth() == 0 {
            // Don't check the top-level directory
            continue;
        }

        if a.depth() != b.depth()
            || a.file_name() != b.file_name()
            || a.file_type() != b.file_type()
        {
            log::trace!(
                "Directory structure mismatch:\n  {}\n  {}",
                a.path().display(),
                b.path().display()
            );

            return Ok(false);
        }

        if a.file_type().is_file() && !file_compare(a.into_path(), b.into_path())? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Compare the contents of two files
fn file_compare<A: AsRef<Path>, B: AsRef<Path>>(a: A, b: B) -> Result<bool> {
    log::trace!("Comparing files:\n  {}\n  {}", a.as_ref().display(), b.as_ref().display());
    let a = File::open(a)?;
    let b = File::open(b)?;

    if a.metadata()?.len() != b.metadata()?.len() {
        log::trace!("File length mismatch");
        return Ok(false);
    }

    let mut a = BufReader::new(a);
    let mut b = BufReader::new(b);

    loop {
        let a_buf = a.fill_buf()?;
        let b_buf = b.fill_buf()?;

        if a_buf.is_empty() && b_buf.is_empty() {
            // Both EOF at same time. Equal files
            return Ok(true);
        }

        let cmp_len = std::cmp::min(a_buf.len(), b_buf.len());
        if cmp_len == 0 {
            // Only 1 EOF. Files are different
            // Note: Because of the file length check above, this should be impossible
            //       except perhaps if a file is actively being changed.
            log::trace!("File length mismatch (race condition?)");
            return Ok(false);
        }

        if a_buf[..cmp_len] != b_buf[..cmp_len] {
            log::trace!("File contents mismatch");
            return Ok(false);
        }

        a.consume(cmp_len);
        b.consume(cmp_len);
    }
}
