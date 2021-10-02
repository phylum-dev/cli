//! This module contains a tokio async runtime to run async functions on
use futures::Future;
use lazy_static::lazy_static;
use tokio::runtime::{Builder, Runtime};

lazy_static! {
    /// Async runtime for spawning async tasks on.
    static ref ASYNC_RUNTIME: Runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Unable to build async runtime.");
}

/// Turn a async task into a synchronous one by running it on tokio
pub fn block_on<F, O>(future: F) -> O
where
    F: Future<Output = O>,
{
    ASYNC_RUNTIME.block_on(future)
}
