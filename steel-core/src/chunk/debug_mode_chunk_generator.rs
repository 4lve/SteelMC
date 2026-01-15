use steel_utils::BlockStateId;

use crate::chunk::{chunk_access::ChunkAccess, chunk_generator::ChunkGenerator};

/// A chunk generator that generates all block states in sequence.
/// Each block state is placed one after another on the Y=5 layer,
/// creating a showcase of all possible block states.
pub struct DebugModeChunkGenerator {
    /// The total number of block states in the registry.
    pub total_states: u16,
    /// The block state id for the ground.
    pub ground: BlockStateId,
}

impl DebugModeChunkGenerator {
    /// Creates a new `DebugModeChunkGenerator` with the given total number of block states and ground block state.
    #[must_use]
    pub fn new(total_states: u16, ground: BlockStateId) -> Self {
        Self {
            total_states,
            ground,
        }
    }

    /// Calculates the block state ID for a given world position.
    fn get_state_for_position(&self, chunk_x: i32, chunk_z: i32, x: i32, z: i32) -> BlockStateId {
        let air = BlockStateId(0);
        // Calculate absolute world position
        let world_x = chunk_x * 16 + x;
        let world_z = chunk_z * 16 + z;

        // Only generate for positive coordinates
        if world_x < 0 || world_z < 0 {
            return air;
        }

        // space between blocks states
        if world_x % 2 != 0 || world_z % 2 != 0 {
            return air;
        }

        let world_x = world_x / 2;
        let world_z = world_z / 2;

        // Calculate grid width (how many blocks wide the showcase is)
        // Using a square root to make it roughly square
        let grid_width = f32::from(self.total_states).sqrt().ceil() as i32;

        if world_x >= grid_width || world_z >= grid_width {
            return air;
        }

        // Calculate block state index from position
        let state_index = world_z * grid_width + world_x;

        if state_index >= 0 && state_index < i32::from(self.total_states) {
            BlockStateId(state_index as u16)
        } else {
            air
        }
    }
}

impl ChunkGenerator for DebugModeChunkGenerator {
    fn create_structures(&self, _chunk: &ChunkAccess) {}

    fn create_biomes(&self, _chunk: &ChunkAccess) {}

    fn fill_from_noise(&self, chunk: &ChunkAccess) {
        // Get chunk position
        let chunk_pos = chunk.pos();
        let chunk_x = chunk_pos.0.x;
        let chunk_z = chunk_pos.0.y;
        let air = BlockStateId(0);

        for x in 0..16 {
            for z in 0..16 {
                chunk.set_relative_block(x, 0, z, self.ground);

                // Place block state at Y=5 if within the showcase grid
                let state_id = self.get_state_for_position(chunk_x, chunk_z, x as i32, z as i32);
                if state_id != air {
                    chunk.set_relative_block(x, 5, z, state_id);
                }
            }
        }
    }

    fn build_surface(&self, _chunk: &ChunkAccess) {}

    fn apply_carvers(&self, _chunk: &ChunkAccess) {}

    fn apply_biome_decorations(&self, _chunk: &ChunkAccess) {}
}
