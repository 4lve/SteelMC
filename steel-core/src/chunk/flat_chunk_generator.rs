use steel_utils::BlockStateId;

use crate::chunk::{chunk_access::ChunkAccess, chunk_generator::ChunkGenerator};

/// A chunk generator that generates a flat world.
pub struct FlatChunkGenerator {
    /// The block state id for bedrock.
    pub bedrock: BlockStateId,
    /// The block state id for dirt.
    pub dirt: BlockStateId,
    /// The block state id for grass blocks.
    pub grass: BlockStateId,
    /// The block state id for torch.
    pub torch: BlockStateId,
}

impl FlatChunkGenerator {
    /// Creates a new `FlatChunkGenerator`.
    #[must_use]
    pub fn new(
        bedrock: BlockStateId,
        dirt: BlockStateId,
        grass: BlockStateId,
        torch: BlockStateId,
    ) -> Self {
        Self {
            bedrock,
            dirt,
            grass,
            torch,
        }
    }
}

impl ChunkGenerator for FlatChunkGenerator {
    fn create_structures(&self, _chunk: &ChunkAccess) {}

    fn create_biomes(&self, _chunk: &ChunkAccess) {}

    fn fill_from_noise(&self, chunk: &ChunkAccess) {
        // Layers:
        // 0: Bedrock
        // 1-2: Dirt
        // 3: Grass Block
        // (Relative to bottom of chunk, assuming 0 is bottom)

        // TODO: Get actual height from level/config?
        // For now assuming standard height behavior where 0 is bottom of the chunk data.

        for x in 0..16 {
            for z in 0..16 {
                // Bedrock at bottom
                chunk.set_relative_block(x, 0, z, self.bedrock);

                // Dirt layers
                chunk.set_relative_block(x, 1, z, self.dirt);
                chunk.set_relative_block(x, 2, z, self.dirt);

                // Grass block
                chunk.set_relative_block(x, 3, z, self.grass);
            }
        }

        chunk.set_relative_block(0, 4, 0, self.torch);
    }

    fn build_surface(&self, _chunk: &ChunkAccess) {}

    fn apply_carvers(&self, _chunk: &ChunkAccess) {}

    fn apply_biome_decorations(&self, _chunk: &ChunkAccess) {}
}
