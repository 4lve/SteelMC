#![allow(clippy::disallowed_types)]
//! Lock wrappers for debug checks and deadlock prevention.
use std::marker::PhantomData;

/// A synchronous mutex.
pub type SyncMutex<T> = SteelMutex<parking_lot::Mutex<T>, T>;
/// A synchronous read-write lock.
pub type SyncRwLock<T> = SteelRwLock<parking_lot::RwLock<T>, T>;

/// An asynchronous mutex.
pub type AsyncMutex<T> = SteelMutex<tokio::sync::Mutex<T>, T>;
/// An asynchronous read-write lock.
pub type AsyncRwLock<T> = SteelRwLock<tokio::sync::RwLock<T>, T>;

/// A trait for locks that can be used with the `SteelLock` wrapper.
pub trait GenericLock<T: ?Sized + Send> {}

impl<T: ?Sized + Send> GenericLock<T> for tokio::sync::Mutex<T> {}

impl<T: ?Sized + Send> GenericLock<T> for parking_lot::Mutex<T> {}

impl<T: ?Sized + Send + Sync> GenericLock<T> for tokio::sync::RwLock<T> {}

impl<T: ?Sized + Send + Sync> GenericLock<T> for parking_lot::RwLock<T> {}

/// A mutex wrapper that allows debug checks to prevent deadlocks.
///
/// Use [`SteelMutex::new_sync`] for synchronous contexts (`parking_lot::Mutex`)
/// or [`SteelMutex::new_async`] for async contexts (`tokio::sync::Mutex`).
#[derive(Debug)]
pub struct SteelMutex<Mutex: GenericLock<T>, T: ?Sized + Send> {
    mutex: Mutex,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Sized + Send> SteelMutex<parking_lot::Mutex<T>, T> {
    /// Creates a new synchronous mutex backed by `parking_lot::Mutex`.
    pub fn new(data: T) -> Self {
        Self {
            mutex: parking_lot::Mutex::new(data),
            _marker: PhantomData,
        }
    }

    /// Acquires the lock, blocking the current thread until it is available.
    pub fn lock(&self) -> parking_lot::MutexGuard<'_, T> {
        self.mutex.lock()
    }

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns `None` if the lock is already held.
    pub fn try_lock(&self) -> Option<parking_lot::MutexGuard<'_, T>> {
        self.mutex.try_lock()
    }
}

impl<T: Sized + Send> SteelMutex<tokio::sync::Mutex<T>, T> {
    /// Creates a new asynchronous mutex backed by `tokio::sync::Mutex`.
    pub fn new(data: T) -> Self {
        Self {
            mutex: tokio::sync::Mutex::new(data),
            _marker: PhantomData,
        }
    }

    /// Acquires the lock, blocking the current thread until it is available.
    ///
    /// # Panics
    /// Panics if called from within an async runtime (use [`lock_async`](Self::lock_async) instead).
    pub fn lock_blocking(&self) -> tokio::sync::MutexGuard<'_, T> {
        self.mutex.blocking_lock()
    }

    /// Acquires the lock asynchronously.
    pub async fn lock_async(&self) -> tokio::sync::MutexGuard<'_, T> {
        self.mutex.lock().await
    }
}

/// An `RwLock` wrapper that allows debug checks to prevent deadlocks.
///
/// Use [`SteelRwLock::new_sync`] for synchronous contexts (`parking_lot::RwLock`)
/// or [`SteelRwLock::new_async`] for async contexts (`tokio::sync::RwLock`).
#[derive(Debug)]
pub struct SteelRwLock<RwLock: GenericLock<T>, T: ?Sized + Send + Sync> {
    rwlock: RwLock,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Sized + Send + Sync> SteelRwLock<parking_lot::RwLock<T>, T> {
    /// Creates a new synchronous `RwLock` backed by `parking_lot::RwLock`.
    pub fn new(data: T) -> Self {
        Self {
            rwlock: parking_lot::RwLock::new(data),
            _marker: PhantomData,
        }
    }

    /// Acquires a read lock, blocking until available.
    ///
    /// Multiple readers can hold the lock simultaneously.
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, T> {
        self.rwlock.read()
    }

    /// Attempts to acquire a read lock without blocking.
    ///
    /// Returns `None` if a write lock is currently held.
    pub fn try_read(&self) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        self.rwlock.try_read()
    }

    /// Acquires a write lock, blocking until available.
    ///
    /// Only one writer can hold the lock at a time.
    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, T> {
        self.rwlock.write()
    }

    /// Attempts to acquire a write lock without blocking.
    ///
    /// Returns `None` if any lock (read or write) is currently held.
    pub fn try_write(&self) -> Option<parking_lot::RwLockWriteGuard<'_, T>> {
        self.rwlock.try_write()
    }
}

impl<T: Sized + Send + Sync> SteelRwLock<tokio::sync::RwLock<T>, T> {
    /// Creates a new asynchronous `RwLock` backed by `tokio::sync::RwLock`.
    pub fn new(data: T) -> Self {
        Self {
            rwlock: tokio::sync::RwLock::new(data),
            _marker: PhantomData,
        }
    }

    /// Acquires a read lock, blocking the current thread until available.
    ///
    /// # Panics
    /// Panics if called from within an async runtime (use [`read_async`](Self::read_async) instead).
    pub fn read_blocking(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.rwlock.blocking_read()
    }

    /// Acquires a read lock asynchronously.
    ///
    /// Multiple readers can hold the lock simultaneously.
    pub async fn read_async(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.rwlock.read().await
    }

    /// Acquires a write lock, blocking the current thread until available.
    ///
    /// # Panics
    /// Panics if called from within an async runtime (use [`write_async`](Self::write_async) instead).
    pub fn write_blocking(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.rwlock.blocking_write()
    }

    /// Acquires a write lock asynchronously.
    ///
    /// Only one writer can hold the lock at a time.
    pub async fn write_async(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.rwlock.write().await
    }
}
