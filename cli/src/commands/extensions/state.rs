//! Shared extension state.

use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use deno_core::OpState;
use futures::future::BoxFuture;
use tokio::sync::OnceCell;

use crate::commands::extensions::{Extension, PhylumApi};

struct OnceFuture<T> {
    future: Cell<Option<BoxFuture<'static, T>>>,
    awaited: OnceCell<T>,
}

impl<T: Unpin> OnceFuture<T> {
    fn new(inner: BoxFuture<'static, T>) -> Self {
        Self { future: Cell::new(Some(inner)), awaited: OnceCell::new() }
    }

    /// # Panics
    ///
    /// This function is not cancellation safe and might panic when used with
    /// [`tokio::select`].
    ///
    /// This is because `get_or_init`'s closure might get cancelled after
    /// [`OnceFuture::future`] was cleared, leaving it empty the next time
    /// someone attempts to initialize the [`OnceCell`].
    async fn get(&self) -> &T {
        self.awaited.get_or_init(|| self.future.take().unwrap()).await
    }
}

// XXX: Holding a `Ref`, `RefMut`, or guard across await points, can cause
// issues when that field is accessed from another extension API
// method.
//
// If a field in [`ExtensionStateInner`] requires mutable access, it should be
// synchronized with an async-safe `Mutex` or `RwLock`, so an attempted access
// to the inner value will cause blocking until the initial guard is resolved.
//
// This also means that if this guard is held across an await point, all other
// access will stall until that future is completed. Avoid holding guards across
// await points, especially when depending on other methods accessing the same
// lock.
//
// Using `RefCell` to avoid locking should only be done when the value is
// private and never held across await points internally. Prefer [`Cell`] if the
// inner value only needs to be replaced.
//
/// Extension state the APIs have access to.
pub struct ExtensionStateInner {
    api: OnceFuture<Result<PhylumApi>>,
    extension: Extension,
}

impl ExtensionStateInner {
    /// # Panics
    ///
    /// This function is not cancellation safe and might panic when used with
    /// [`tokio::select`].
    pub async fn api(&self) -> Result<&PhylumApi> {
        self.api.get().await.as_ref().map_err(|e| anyhow!("{:?}", e))
    }

    /// Returns a reference to the extension.
    pub fn extension(&self) -> &Extension {
        &self.extension
    }
}

/// Extension state wrapper.
///
/// This wrapper allows safely retrieving the extension state from Deno's
/// `Rc<RefCell<OpState>>`, without running the risk of holding a reference to
/// the [`OpState`] across await points.
#[derive(Clone)]
pub struct ExtensionState(Rc<ExtensionStateInner>);

impl ExtensionState {
    pub fn new(api: BoxFuture<'static, Result<PhylumApi>>, extension: Extension) -> Self {
        let api = OnceFuture::new(api);
        Self(Rc::new(ExtensionStateInner { api, extension }))
    }
}

impl From<Rc<RefCell<OpState>>> for ExtensionState {
    fn from(op_state: Rc<RefCell<OpState>>) -> Self {
        op_state.borrow().borrow::<ExtensionState>().clone()
    }
}

impl Deref for ExtensionState {
    type Target = ExtensionStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
