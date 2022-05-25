use std::fs::read_to_string;
use std::path::Path;

use phylum_types::types::package::PackageDescriptor;
use phylum_types::types::package::PackageType;

mod csharp;
mod java;
mod javascript;
mod parsers;
mod python;
mod ruby;

pub use csharp::CSProj;
pub use java::{GradleLock, Pom};
pub use javascript::{PackageLock, YarnLock};
pub use python::{PipFile, Poetry, PyRequirements};
pub use ruby::GemLock;

pub type ParseResult = anyhow::Result<Vec<PackageDescriptor>>;

pub trait Parser {
    /// Parse from a string
    fn parse(&self, data: &str) -> ParseResult;
    /// Indicate the type of packages parsed by this parser
    fn package_type(&self) -> PackageType;
}

/// Parse a file with the given parser.
pub fn parse_file<T: Parser, P: AsRef<Path>>(parser: T, path: P) -> ParseResult {
    let data = read_to_string(path)?;
    parser.parse(&data)
}
