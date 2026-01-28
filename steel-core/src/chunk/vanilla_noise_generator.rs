//! Vanilla-accurate noise chunk generator using the data-driven noise router.
//!
//! This generator uses the same density function component stack as vanilla Minecraft
//! for accurate terrain generation with proper cell-based interpolation, aquifers,
//! and ore vein generation.
//!
//! # Architecture
//!
//! The generation uses a two-stage noise router architecture:
//! 1. **ProtoNoiseRouters**: Built once per world from seed, contains initialized noise samplers
//! 2. **ChunkNoiseGenerator**: Built per chunk, handles interpolation and block sampling
//!
//! # Cell-Based Interpolation
//!
//! Instead of sampling density at every block (computationally expensive), the generator
//! uses **trilinear interpolation** between cell corners:
//!
//! - **Horizontal cell size**: 4 blocks (4 cells per chunk horizontally)
//! - **Vertical cell size**: 8 blocks (48 cells per chunk vertically)
//! - **Total cells per chunk**: 4 × 48 × 4 = 768 cells
//!
//! The interpolation loop structure:
//! ```text
//! for cell_x:
//!     sample_end_density()      // Sample next X column
//!     for cell_z:
//!         for cell_y (top to bottom):
//!             on_sampled_cell_corners()  // Cache 8 corners
//!             for local_y:
//!                 interpolate_y(delta)
//!                 for local_x:
//!                     interpolate_x(delta)
//!                     for local_z:
//!                         interpolate_z(delta)
//!                         sample_block_state()  // Final block
//!     swap_buffers()  // Reuse end as start
//! ```
//!
//! # Block Sampling Chain
//!
//! The [`ChainedBlockStateSampler`] determines the final block:
//! 1. **OreVeinSampler**: Check for large copper/iron veins
//! 2. **AquiferSampler**: Check for water/lava placement
//! 3. **Default**: Stone if density > 0, air otherwise
//!
//! [`ChainedBlockStateSampler`]: steel_utils::noise_router::ChainedBlockStateSampler

// Uses coordinate variables (cell_x, cell_y, cell_z, start_cell_z, etc.)
#![allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]

use steel_utils::{
    noise::floor_div,
    noise_router::{
        FluidLevel, FluidLevelSampler, OVERWORLD_BASE_NOISE_ROUTER, StandardChunkFluidLevelSampler,
        proto_noise_router::ProtoNoiseRouters,
    },
};

use crate::chunk::{chunk_access::ChunkAccess, chunk_generator::ChunkGenerator};

use super::chunk_noise_generator::{ChunkNoiseGenerator, GenerationShapeConfig, TerrainBlocks};
use super::random_config::WorldRandomConfig;

/// Sea level for overworld.
const SEA_LEVEL: i32 = 63;

/// A vanilla-accurate chunk generator using the noise router with aquifers and ore veins.
pub struct VanillaNoiseGenerator {
    /// Proto noise routers (built once from seed).
    proto_routers: ProtoNoiseRouters,
    /// Generation shape.
    shape: GenerationShapeConfig,
    /// Random configuration.
    random_config: WorldRandomConfig,
    /// Terrain block state IDs.
    blocks: TerrainBlocks,
}

impl VanillaNoiseGenerator {
    /// Creates a new generator with the given seed and block states.
    #[must_use]
    pub fn new(seed: u64, blocks: TerrainBlocks) -> Self {
        let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, seed);
        let shape = GenerationShapeConfig::overworld();
        let random_config = WorldRandomConfig::new(seed);

        Self {
            proto_routers,
            shape,
            random_config,
            blocks,
        }
    }

    /// Get the world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.random_config.seed
    }

    /// Creates the fluid level sampler for overworld.
    fn create_fluid_level_sampler(&self) -> FluidLevelSampler {
        FluidLevelSampler::Standard(StandardChunkFluidLevelSampler::new(
            FluidLevel::new(SEA_LEVEL, self.blocks.water),
            FluidLevel::new(-54, self.blocks.lava),
            -54,
        ))
    }
}

impl ChunkGenerator for VanillaNoiseGenerator {
    fn create_structures(&self, _chunk: &ChunkAccess) {
        // TODO: Structure generation
    }

    fn create_biomes(&self, _chunk: &ChunkAccess) {
        // TODO: Biome generation
    }

    fn fill_from_noise(&self, chunk: &ChunkAccess) {
        let chunk_pos = chunk.pos();
        let base_x = chunk_pos.0.x * 16;
        let base_z = chunk_pos.0.y * 16;

        let h_count = i32::from(self.shape.horizontal_cell_block_count());
        let v_count = i32::from(self.shape.vertical_cell_block_count());
        let horizontal_cells = 16 / h_count;
        let vertical_cell_count = floor_div(i32::from(self.shape.height), v_count) as usize;

        let delta_y_step = 1.0 / f64::from(v_count);
        let delta_xz_step = 1.0 / f64::from(h_count);

        let min_y = i32::from(self.shape.min_y);

        // Create fluid level sampler
        let fluid_level_sampler = self.create_fluid_level_sampler();

        // Create chunk noise generator with aquifers and ore veins
        let mut generator = ChunkNoiseGenerator::new(
            &self.proto_routers.noise,
            &self.proto_routers.surface_estimator,
            &self.random_config,
            horizontal_cells as usize,
            base_x,
            base_z,
            &self.shape,
            fluid_level_sampler,
            &self.blocks,
            true, // enable_aquifers
            true, // enable_ore_veins
        );

        // Sample start density column
        generator.sample_start_density();

        for cell_x in 0..horizontal_cells {
            // Sample end density column
            generator.sample_end_density(cell_x);

            let sample_start_x = (generator.start_cell_pos_x() + cell_x) * h_count;
            let block_x_base = base_x + cell_x * h_count;

            for cell_z in 0..horizontal_cells {
                let sample_start_z = (generator.start_cell_pos_z() + cell_z) * h_count;
                let block_z_base = base_z + cell_z * h_count;

                for cell_y in (0..vertical_cell_count as i32).rev() {
                    // Notify generator about cell corners and fill cell caches
                    generator.on_sampled_cell_corners(cell_x, cell_y, cell_z);

                    let sample_start_y = (generator.minimum_cell_y() + cell_y) * v_count;

                    for local_y in (0..v_count).rev() {
                        let block_y = sample_start_y + local_y;
                        generator.interpolate_y(f64::from(local_y) * delta_y_step);

                        for local_x in 0..h_count {
                            generator.interpolate_x(f64::from(local_x) * delta_xz_step);
                            let block_x = block_x_base + local_x;

                            for local_z in 0..h_count {
                                generator.interpolate_z(f64::from(local_z) * delta_xz_step);
                                let block_z = block_z_base + local_z;

                                let cell_offset_x = local_x;
                                let cell_offset_y = block_y - sample_start_y;
                                let cell_offset_z = local_z;

                                // Sample block state using the chained sampler
                                let sampled_block = generator.sample_block_state(
                                    sample_start_x,
                                    sample_start_y,
                                    sample_start_z,
                                    cell_offset_x,
                                    cell_offset_y,
                                    cell_offset_z,
                                );

                                // Determine final block
                                // Bedrock and deepslate are placed later by surface rules
                                let local_y_idx = (block_y - min_y) as usize;
                                let local_x_idx = (block_x - base_x) as usize;
                                let local_z_idx = (block_z - base_z) as usize;

                                let block = if let Some(block) = sampled_block {
                                    // Use sampled block (water, lava, ore, etc.)
                                    block
                                } else {
                                    // Solid block (sampler returned None)
                                    self.blocks.stone
                                };

                                // Skip air blocks
                                if block == self.blocks.air {
                                    continue;
                                }

                                chunk.set_relative_block(
                                    local_x_idx,
                                    local_y_idx,
                                    local_z_idx,
                                    block,
                                );
                            }
                        }
                    }
                }
            }

            // Swap buffers for next column
            generator.swap_buffers();
        }
    }

    fn build_surface(&self, _chunk: &ChunkAccess) {
        // TODO: Surface generation
    }

    fn apply_carvers(&self, _chunk: &ChunkAccess) {
        // TODO: Cave carvers
    }

    fn apply_biome_decorations(&self, _chunk: &ChunkAccess) {
        // TODO: Trees, ores, etc.
    }
}
