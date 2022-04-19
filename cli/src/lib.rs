pub mod api;
pub mod app;
pub mod auth;
pub mod commands;
pub mod config;
pub mod filter;
pub mod lockfiles;
pub mod print;
pub mod prompt;
pub mod render;
pub mod summarize;
#[cfg(test)]
mod test;
pub mod types;
pub mod update;

#[cfg(test)]
#[allow(unused_imports)]
// Enable logging for ALL doc & local tests
use test::logging;

pub use reqwest::Error;

/// Cargo crate version.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
