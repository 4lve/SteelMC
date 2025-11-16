use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

/// A wrapper that allows us to do debug checks on the lock. To prevent deadlocks.
#[derive(Debug)]
pub struct SteelRwLock<T>(RwLock<T>);

#[allow(missing_docs)]
impl<T> SteelRwLock<T> {
    pub fn new(inner: T) -> Self {
        Self(RwLock::new(inner))
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read().await
    }

    pub fn blocking_read(&self) -> RwLockReadGuard<'_, T> {
        self.0.blocking_read()
    }

    pub fn try_read(&self) -> Result<RwLockReadGuard<'_, T>, TryLockError> {
        self.0.try_read()
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write().await
    }

    pub fn blocking_write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.blocking_write()
    }

    pub fn try_write(&self) -> Result<RwLockWriteGuard<'_, T>, TryLockError> {
        self.0.try_write()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}
