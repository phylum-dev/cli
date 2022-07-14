//! Shared extension state.

use std::cell::RefCell;
use std::rc::Rc;
use std::ops::Deref;

use anyhow::{anyhow, Result};
use deno_runtime::deno_core::OpState;
use futures::future::BoxFuture;
use tokio::sync::Mutex;

use crate::commands::extensions::permissions::Permissions;
use crate::commands::extensions::PhylumApi;

/// Holds either an unawaited, boxed `Future`, or the result of awaiting the
/// future.
enum OnceFuture<T: Unpin> {
    Future(BoxFuture<'static, T>),
    Awaited(T),
}

impl<T: Unpin> OnceFuture<T> {
    fn new(inner: BoxFuture<'static, T>) -> Self {
        OnceFuture::Future(inner)
    }

    async fn get(&mut self) -> &T {
        match *self {
            OnceFuture::Future(ref mut inner) => {
                *self = OnceFuture::Awaited(inner.await);
                match *self {
                    OnceFuture::Future(..) => unreachable!(),
                    OnceFuture::Awaited(ref mut inner) => inner,
                }
            },
            OnceFuture::Awaited(ref mut inner) => inner,
        }
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
    pub permissions: Permissions,

    api: Mutex<OnceFuture<Result<Rc<PhylumApi>>>>,
}

impl ExtensionStateInner {
    pub async fn api(&self) -> Result<Rc<PhylumApi>> {
        // This mutex guard is only useful for synchronizing mutable access while
        // awaiting the PhylumApi future. Subsequent access to the API is
        // immediate and will not hold the Mutex for extended periods of time.
        let mut guard = self.api.lock().await;
        Ok(guard.get().await.as_ref().map_err(|e| anyhow!("{:?}", e))?.clone())
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
    pub fn new(api: BoxFuture<'static, Result<PhylumApi>>, permissions: Permissions) -> Self {
        let api = Mutex::new(OnceFuture::new(Box::pin(async { api.await.map(Rc::new) })));
        Self(Rc::new(ExtensionStateInner { permissions, api }))
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
