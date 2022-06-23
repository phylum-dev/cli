//! Self-update functions
//!
//! Self-update is currently supported on macos and Linux. All other platforms
//! are unsupported and will display an error when `phylum update` is run. For
//! Windows support, see issue #221

#[cfg(unix)]
pub use self::unix::*;
#[cfg(not(unix))]
pub use self::unsupported::*;

#[cfg(unix)]
mod unix;
#[cfg(not(unix))]
mod unsupported;
