//! Test world implementation using the real steel-core World.
//!
//! This module provides a test world that wraps the real `Arc<World>` from steel-core,
//! configured with RAM-only storage for instant chunk creation without disk I/O.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use flint_core::test_spec::Block as FlintBlock;
use flint_steel::traits::BlockData;
use flint_steel::{BlockPos as FlintBlockPos, FlintPlayer, FlintWorld};
use steel_core::chunk::empty_chunk_generator::EmptyChunkGenerator;
use steel_core::chunk::world_gen_context::ChunkGeneratorType;
use steel_core::world::{World, WorldConfig, WorldStorageConfig};
use steel_registry::vanilla_dimension_types::OVERWORLD;
use steel_utils::{BlockPos, types::UpdateFlags};

use crate::convert::{flint_block_to_state_id, flint_pos_to_steel, state_id_to_block_data};
use crate::player::SteelTestPlayer;
use crate::runtime;

/// Test world implementation using the real steel-core World.
///
/// This wraps an `Arc<World>` configured with RAM-only storage:
/// - Chunks are created empty (all air) on-demand
/// - No disk I/O, no chunk generation delay
/// - Full block behavior system (neighbors, shapes, etc.)
/// - Real tick processing
pub struct SteelTestWorld {
    /// The underlying steel-core world.
    world: Arc<World>,
    /// Current tick count (for `FlintWorld` trait).
    tick: AtomicU64,
}

impl SteelTestWorld {
    /// Creates a new test world with RAM-only storage.
    ///
    /// The world uses the overworld dimension type and starts with seed 0.
    /// All chunks are created empty on-demand.
    ///
    /// # Panic
    /// shouldn't panic only something is completely broken and then it is ok
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn new() -> Self {
        let rt = runtime();

        // Create world with RAM-only storage
        let config = WorldConfig {
            storage: WorldStorageConfig::RamOnlyEmpty,
            generator: Arc::new(ChunkGeneratorType::Empty(EmptyChunkGenerator::new()))
        };

        let dimension = OVERWORLD;

        // Block on async world creation
        let world = rt
            .block_on(async { World::new_with_config(rt.clone(), dimension, 0, config).await })
            .expect("Failed to create test world");

        Self {
            world,
            tick: AtomicU64::new(0),
        }
    }

    /// Gets a reference to the underlying steel-core world.
    #[must_use]
    pub fn inner(&self) -> &Arc<World> {
        &self.world
    }

    /// Ensures the chunk containing the given block position is loaded.
    fn ensure_chunk_at(&self, pos: &BlockPos) {
        use steel_utils::ChunkPos;

        let chunk_x = pos.x() >> 4;
        let chunk_z = pos.z() >> 4;
        let chunk_pos = ChunkPos::new(chunk_x, chunk_z);

        self.world.chunk_map.ensure_chunk_loaded(&chunk_pos);
    }
}

impl Default for SteelTestWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl FlintWorld for SteelTestWorld {
    fn do_tick(&mut self) {
        let tick_count = self.tick.fetch_add(1, Ordering::SeqCst);

        // Run a real world tick
        // Note: For testing we run with `runs_normally = true`
        self.world.tick_b(tick_count, true);
    }

    fn current_tick(&self) -> u64 {
        self.tick.load(Ordering::SeqCst)
    }

    fn get_block(&self, pos: FlintBlockPos) -> BlockData {
        let steel_pos = flint_pos_to_steel(pos);

        // Ensure the chunk is loaded (for RAM storage this creates empty chunks)
        self.ensure_chunk_at(&steel_pos);

        let state = self.world.get_block_state(&steel_pos);
        state_id_to_block_data(state)
    }

    fn set_block(&mut self, pos: FlintBlockPos, block: &FlintBlock) {
        let Some(state_id) = flint_block_to_state_id(block) else {
            log::warn!("Unknown block: {} - skipping placement", block.id);
            return;
        };

        let steel_pos = flint_pos_to_steel(pos);

        // Ensure the chunk is loaded before setting blocks
        self.ensure_chunk_at(&steel_pos);

        // Use the real World::set_block which handles:
        // - Neighbor updates
        // - Shape updates
        // - Block behavior callbacks (on_place, etc.)
        self.world
            .set_block(steel_pos, state_id, UpdateFlags::UPDATE_ALL);
    }

    fn create_player(&mut self) -> Box<dyn FlintPlayer> {
        Box::new(SteelTestPlayer::new(self.world.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_registries;
    use rustc_hash::FxHashMap;

    #[test]
    fn test_world_creation() {
        init_test_registries();
        let world = SteelTestWorld::new();
        assert_eq!(world.current_tick(), 0);
    }

    #[test]
    fn test_world_tick() {
        init_test_registries();
        let mut world = SteelTestWorld::new();
        assert_eq!(world.current_tick(), 0);

        world.do_tick();
        assert_eq!(world.current_tick(), 1);

        world.do_tick();
        world.do_tick();
        assert_eq!(world.current_tick(), 3);
    }

    #[test]
    fn test_get_air_by_default() {
        init_test_registries();
        let world = SteelTestWorld::new();
        let block = world.get_block([0, 64, 0]);
        // Empty chunks are filled with air (or void_air depending on implementation)
        assert!(
            block.id == "minecraft:air" || block.id == "minecraft:void_air",
            "Expected air or void_air, got: {}",
            block.id
        );
    }

    #[test]
    fn test_set_and_get_block() {
        init_test_registries();
        let mut world = SteelTestWorld::new();

        let stone = FlintBlock {
            id: "minecraft:stone".to_string(),
            properties: FxHashMap::default(),
        };

        world.set_block([0, 64, 0], &stone);

        let retrieved = world.get_block([0, 64, 0]);
        assert_eq!(retrieved.id, "minecraft:stone");
    }

    #[test]
    fn test_set_air_clears_block() {
        init_test_registries();
        let mut world = SteelTestWorld::new();

        // Place a block
        let stone = FlintBlock {
            id: "minecraft:stone".to_string(),
            properties: FxHashMap::default(),
        };
        world.set_block([0, 64, 0], &stone);

        let retrieved = world.get_block([0, 64, 0]);
        assert_eq!(retrieved.id, "minecraft:stone");

        // Remove with air
        let air = FlintBlock {
            id: "minecraft:air".to_string(),
            properties: FxHashMap::default(),
        };
        world.set_block([0, 64, 0], &air);

        let retrieved = world.get_block([0, 64, 0]);
        // Accept both air and void_air as valid "cleared" states
        assert!(
            retrieved.id == "minecraft:air" || retrieved.id == "minecraft:void_air",
            "Expected air or void_air, got: {}",
            retrieved.id
        );
    }
}
