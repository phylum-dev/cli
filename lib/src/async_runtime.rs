//! This module contains a tokio async runtime to run async functions on
use lazy_static::lazy_static;
use tokio::runtime::{Builder, Runtime};

lazy_static! {
    /// Async runtime for spawning async tasks on.
    pub static ref ASYNC_RUNTIME: Runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Unable to build async runtime.");
}
