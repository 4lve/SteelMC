#![allow(clippy::disallowed_types)]
//! Lock wrappers for debug checks and deadlock prevention.

/// A synchronous mutex.
pub type SyncMutex<T> = parking_lot::Mutex<T>;
/// A synchronous read-write lock.
pub type SyncRwLock<T> = parking_lot::RwLock<T>;

/// An asynchronous mutex.
pub type AsyncMutex<T> = tokio::sync::Mutex<T>;
/// An asynchronous read-write lock.
pub type AsyncRwLock<T> = tokio::sync::RwLock<T>;
