use std::io;
use std::marker::Sized;
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
pub use java::{GradleDeps, Pom};
pub use javascript::{PackageLock, YarnLock};
pub use python::{PipFile, Poetry, PyRequirements};
pub use ruby::GemLock;

pub type ParseResult = anyhow::Result<Vec<PackageDescriptor>>;

pub trait Parseable {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(Self::from_string(std::fs::read_to_string(filename)?))
    }

    fn from_string(text: String) -> Self
    where
        Self: Sized;

    fn parse(&self) -> ParseResult;

    fn package_type() -> PackageType;
}
