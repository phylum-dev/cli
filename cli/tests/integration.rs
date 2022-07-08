#[path = "common/mod.rs"]
mod common;

#[cfg(feature = "end-to-end-tests")]
#[path = "end_to_end/mod.rs"]
mod end_to_end;

#[cfg(feature = "extensions")]
#[path = "extensions/mod.rs"]
mod extensions;
