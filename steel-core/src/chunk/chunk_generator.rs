//! This module contains the `ChunkGenerator` trait, which is used to generate chunks.

use crate::chunk::chunk_access::ChunkAccess;
use enum_dispatch::enum_dispatch;

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

/// Blanket implementation for boxed chunk generators.
/// This allows `Box<T>` to be used in `enum_dispatch` enums.
impl<T: ChunkGenerator + ?Sized> ChunkGenerator for Box<T> {
    fn create_structures(&self, chunk: &ChunkAccess) {
        (**self).create_structures(chunk);
    }

    fn create_biomes(&self, chunk: &ChunkAccess) {
        (**self).create_biomes(chunk);
    }

    fn fill_from_noise(&self, chunk: &ChunkAccess) {
        (**self).fill_from_noise(chunk);
    }

    fn build_surface(&self, chunk: &ChunkAccess) {
        (**self).build_surface(chunk);
    }

    fn apply_carvers(&self, chunk: &ChunkAccess) {
        (**self).apply_carvers(chunk);
    }

    fn apply_biome_decorations(&self, chunk: &ChunkAccess) {
        (**self).apply_biome_decorations(chunk);
    }
}
