//! This module contains the `ChunkGenerator` trait, which is used to generate chunks.
use crate::chunk::proto_chunk::ProtoChunk;

/// A trait for generating chunks.
pub trait ChunkGenerator {
    // TODO: Look into making the proto chunks be chunk holders instead, otherwise it holdsd the lock for the whole chunk for the whole generation process.

    /// Creates the structures in a chunk.
    fn create_structures(&self, proto_chunk: &mut ProtoChunk);

    /// Creates the biomes in a chunk.
    fn create_biomes(&self, proto_chunk: &mut ProtoChunk);

    /// Fills the chunk with noise.
    fn fill_from_noise(&self, proto_chunk: &mut ProtoChunk);

    /// Builds the surface of the chunk.
    fn build_surface(&self, proto_chunk: &mut ProtoChunk);

    /// Applies carvers to the chunk.
    fn apply_carvers(&self, proto_chunk: &mut ProtoChunk);

    /// Applies biome decorations to the chunk.
    fn apply_biome_decorations(&self, proto_chunk: &mut ProtoChunk);
}
