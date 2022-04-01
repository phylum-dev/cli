//! Self-update functions
//!
//! Self-update is currently only supported on macos and Linux. All other platforms are unsupported
//! and will display an error when `phylum update` is run.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::*;

#[cfg(not(unix))]
mod unsupported;
#[cfg(not(unix))]
pub use self::unsupported::*;
