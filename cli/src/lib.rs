pub mod api;
pub mod app;
pub mod auth;
pub mod commands;
pub mod config;
#[cfg(feature = "extensions")]
pub mod deno;
pub mod dirs;
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

pub use reqwest::Error;
#[cfg(test)]
#[allow(unused_imports)]
// Enable logging for ALL doc & local tests
use test::logging;
