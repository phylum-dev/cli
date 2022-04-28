use std::error::Error;
use std::io;
use std::marker::Sized;
use std::path::Path;

use phylum_types::types::package::PackageDescriptor;

mod csharp;
mod java;
mod javascript;
mod parsers;
mod python;
mod ruby;

pub use csharp::CSProj;
pub use java::{GradleDeps, Pom};
pub use javascript::{PackageLock, YarnLock};
pub use python::{PipFile, Poetry, PyRequirements};
pub use ruby::GemLock;

pub type ParseResult = Result<Vec<PackageDescriptor>, Box<dyn Error>>;

pub trait Parseable {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized;
    fn parse(&self) -> ParseResult;
}
