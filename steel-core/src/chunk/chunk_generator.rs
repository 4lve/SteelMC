//! This module contains the `ChunkGenerator` trait, which is used to generate chunks.

use crate::chunk::chunk_access::ChunkAccess;
use enum_dispatch::enum_dispatch;
use std::sync::Arc;

/// A guard that provides access to a chunk.
/// This is a wrapper around arc_swap::Guard to provide a more ergonomic API.
pub struct ChunkGuard(pub Arc<ChunkAccess>);

impl std::ops::Deref for ChunkGuard {
    type Target = ChunkAccess;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ChunkGuard {
    /// Creates a new ChunkGuard from an arc_swap Guard
    pub fn new(guard: arc_swap::Guard<Option<Arc<ChunkAccess>>>) -> Self {
        let chunk = guard.as_ref().expect("Chunk should be Some").clone();
        Self(chunk)
    }
}

/// A trait for generating chunks.
#[enum_dispatch]
pub trait ChunkGenerator: Send + Sync {
    /// Creates the structures in a chunk.
    fn create_structures(&self, chunk: &ChunkAccess);

    /// Creates the biomes in a chunk.
    fn create_biomes(&self, chunk: &ChunkAccess);

    /// Fills the chunk with noise.
    fn fill_from_noise(&self, chunk: &ChunkAccess);

    /// Builds the surface of the chunk.
    fn build_surface(&self, chunk: &ChunkAccess);

    /// Applies carvers to the chunk.
    fn apply_carvers(&self, chunk: &ChunkAccess);

    /// Applies biome decorations to the chunk.
    fn apply_biome_decorations(&self, chunk: &ChunkAccess);
}
