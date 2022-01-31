pub mod app;
pub mod api;
pub mod auth;
pub mod config;
pub mod filter;
pub mod lockfiles;
pub mod render;
pub mod summarize;
#[cfg(test)]
mod test;
pub mod types;
pub mod update;
pub mod utils;

#[cfg(test)]
#[allow(unused_imports)]
// Enable logging for ALL doc & local tests
use test::logging;

#[macro_use]
extern crate log;

//#[macro_use]
//extern crate prettytable;
pub use reqwest::Error;
