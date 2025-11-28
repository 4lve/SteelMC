//! This module contains the `ChunkGenerator` trait, which is used to generate chunks.
use std::ops::{Deref, DerefMut};

use crate::chunk::{chunk_access::ChunkAccess, proto_chunk::ProtoChunk};
use parking_lot::{RwLock as ParkingRwLock, RwLockWriteGuard};

pub struct YieldableGuard<'a> {
    mutex: &'a ParkingRwLock<Option<ChunkAccess>>,
    guard: Option<RwLockWriteGuard<'a, Option<ChunkAccess>>>,
}

impl<'a> YieldableGuard<'a> {
    pub fn new(mutex: &'a ParkingRwLock<Option<ChunkAccess>>) -> Self {
        Self {
            mutex,
            guard: Some(mutex.write()),
        }
    }

    /// Temporarily drops the lock, runs the provided closure, and re-acquires the lock immediately after.
    ///
    /// This is safe because `&mut self` guarantees no other references to the
    /// underlying data exist while this method runs.
    pub fn yield_lock<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce() -> R,
    {
        // 1. Drop the lock strictly before running the closure
        self.guard = None; // This drops the MutexGuard immediately

        // 2. Run the external work
        // The lock is completely free here.
        let result = func();

        // 3. Re-acquire the lock immediately
        self.guard = Some(self.mutex.write());

        result
    }
}

// Implement Deref so you can treat it exactly like a normal reference to T
impl Deref for YieldableGuard<'_> {
    type Target = ChunkAccess;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: This unwrap is safe because `guard` is only None
        // *inside* `yield_lock`, where `&mut self` prevents calling `deref`.
        self.guard.as_deref().unwrap().as_ref().unwrap()
    }
}

// Implement DerefMut for mutable access
impl DerefMut for YieldableGuard<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_deref_mut().unwrap().as_mut().unwrap()
    }
}

/// A trait for generating chunks.
pub trait ChunkGenerator: Send + Sync {
    // TODO: Look into making the proto chunks be chunk holders instead, otherwise it holdsd the lock for the whole chunk for the whole generation process.

    /// Creates the structures in a chunk.
    fn create_structures(&self, proto_chunk: &mut ProtoChunk);

    /// Creates the biomes in a chunk.
    fn create_biomes(&self, proto_chunk: &mut ProtoChunk);

    /// Fills the chunk with noise.
    fn fill_from_noise(&self, yieldable_guard: &mut YieldableGuard);

    /// Builds the surface of the chunk.
    fn build_surface(&self, proto_chunk: &mut ProtoChunk);

    /// Applies carvers to the chunk.
    fn apply_carvers(&self, proto_chunk: &mut ProtoChunk);

    /// Applies biome decorations to the chunk.
    fn apply_biome_decorations(&self, proto_chunk: &mut ProtoChunk);
}

/// A simple chunk generator that does nothing.
pub struct SimpleChunkGenerator;

impl ChunkGenerator for SimpleChunkGenerator {
    fn create_structures(&self, _proto_chunk: &mut ProtoChunk) {}
    fn create_biomes(&self, _proto_chunk: &mut ProtoChunk) {}
    fn fill_from_noise(&self, _yieldable_guard: &mut YieldableGuard) {}
    fn build_surface(&self, _proto_chunk: &mut ProtoChunk) {}
    fn apply_carvers(&self, _proto_chunk: &mut ProtoChunk) {}
    fn apply_biome_decorations(&self, _proto_chunk: &mut ProtoChunk) {}
}
