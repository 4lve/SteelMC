//! Aquifer sampler for cave and underground water/lava generation.
//!
//! This module contains the aquifer sampling system that determines where
//! water and lava appear in caves and underground areas.

// Uses coordinate variables (cell_x, cell_y, cell_z, etc.)
#![allow(clippy::similar_names, clippy::too_many_lines)]

use enum_dispatch::enum_dispatch;

use crate::noise::{clamped_map, floor_div, map};
use crate::random::{PositionalRandom, Random, RandomSplitter};
use crate::BlockStateId;

use super::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use super::chunk_noise_router::ChunkNoiseRouter;
use super::density_function::{NoisePos, UnblendedNoisePos};
use super::fluid_level::{FluidLevel, FluidLevelSampler, FluidLevelSamplerImpl};
use super::surface_height_sampler::SurfaceHeightEstimateSampler;

/// Minimum height cell value used as sentinel for "no aquifer".
const MIN_HEIGHT_CELL: i32 = i32::MIN;

/// Chunk position offsets for 13-chunk sampling pattern.
const CHUNK_POS_OFFSETS: [(i8, i8); 13] = [
    (0, 0),   // center
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1), // left section
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (1, 0), // center section
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1), // right section
];

/// Trait for aquifer sampler implementations.
#[enum_dispatch]
pub trait AquiferSamplerImpl {
    /// Applies the aquifer sampler to determine the block at a position.
    ///
    /// Returns `Some(block)` if the aquifer determines a specific block (water, lava, or stone),
    /// or `None` if the default solid block should be used.
    fn apply(
        &mut self,
        router: &mut ChunkNoiseRouter,
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> Option<BlockStateId>;
}

/// Aquifer sampler variants.
#[enum_dispatch(AquiferSamplerImpl)]
pub enum AquiferSampler {
    /// Simple sea-level based aquifer (no underground water pockets).
    SeaLevel(SeaLevelAquiferSampler),
    /// Full world aquifer with underground water/lava pockets.
    World(WorldAquiferSampler),
}

/// Block state IDs needed by the aquifer sampler.
#[derive(Clone)]
pub struct AquiferBlocks {
    /// Water block state.
    pub water: BlockStateId,
    /// Lava block state.
    pub lava: BlockStateId,
    /// Air block state.
    pub air: BlockStateId,
}

/// Simple aquifer sampler that uses sea level for fluid placement.
pub struct SeaLevelAquiferSampler {
    level_sampler: FluidLevelSampler,
    blocks: AquiferBlocks,
}

impl SeaLevelAquiferSampler {
    /// Creates a new sea level aquifer sampler.
    #[must_use]
    pub const fn new(level_sampler: FluidLevelSampler, blocks: AquiferBlocks) -> Self {
        Self {
            level_sampler,
            blocks,
        }
    }
}

impl AquiferSamplerImpl for SeaLevelAquiferSampler {
    fn apply(
        &mut self,
        router: &mut ChunkNoiseRouter,
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        _height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> Option<BlockStateId> {
        let density = router.final_density(pos, sample_options);
        if density > 0.0 {
            None // Solid block
        } else {
            let level = self.level_sampler.get_fluid_level(pos.x(), pos.y(), pos.z());
            Some(level.get_block(pos.y(), self.blocks.air))
        }
    }
}

/// Converts block coordinate to 16-block cell coordinate.
#[inline]
fn local_xz(xz: i32) -> i32 {
    floor_div(xz, 16)
}

/// Converts block coordinate to 12-block cell coordinate.
#[inline]
fn local_y(y: i32) -> i32 {
    floor_div(y, 12)
}

/// Calculates index into packed positions array.
#[inline]
fn packed_position_index(x: usize, y: usize, z: usize, dim_y: usize, dim_z: usize) -> usize {
    (x * dim_z + z) * dim_y + y
}

/// Full world aquifer sampler with underground water/lava pockets.
pub struct WorldAquiferSampler {
    /// Fluid level sampler for default levels.
    fluid_level_sampler: FluidLevelSampler,
    /// Block states for water/lava/air.
    blocks: AquiferBlocks,
    /// Starting X coordinate in 16-block cells.
    start_x: i32,
    /// Starting Y coordinate in 12-block cells.
    start_y: i32,
    /// Starting Z coordinate in 16-block cells.
    start_z: i32,
    /// Y dimension size.
    size_y: usize,
    /// Z dimension size.
    size_z: usize,
    /// Cached fluid levels (lazily computed).
    levels: Box<[Option<FluidLevel>]>,
    /// Pre-computed random positions packed as i64.
    packed_positions: Box<[i64]>,
}

impl WorldAquiferSampler {
    /// Creates a new world aquifer sampler.
    #[must_use]
    pub fn new(
        chunk_x: i32,
        chunk_z: i32,
        random_deriver: &RandomSplitter,
        minimum_y: i8,
        height: u16,
        fluid_level_sampler: FluidLevelSampler,
        blocks: AquiferBlocks,
    ) -> Self {
        // Convert chunk coords to 16-block cells
        let start_x = local_xz(chunk_x * 16) - 1;
        let start_z = local_xz(chunk_z * 16) - 1;

        let max_y = i32::from(minimum_y) + i32::from(height);
        let start_y = local_y(i32::from(minimum_y)) - 1;
        let end_y = local_y(max_y) + 1;

        let size_x = local_xz(16) + 3; // 4
        let size_y = (end_y - start_y + 1) as usize;
        let size_z = local_xz(16) + 3; // 4

        let total_size = size_x as usize * size_y * size_z as usize;

        // Initialize packed positions with random offsets
        let mut packed_positions = Vec::with_capacity(total_size);

        for x_cell in 0..size_x {
            let abs_x = start_x + x_cell;
            for z_cell in 0..size_z as i32 {
                let abs_z = start_z + z_cell;
                for y_cell in 0..size_y as i32 {
                    let abs_y = start_y + y_cell;

                    let mut random = random_deriver.at(abs_x, abs_y, abs_z);

                    // Random offset within cell
                    let rand_x = abs_x * 16 + random.next_i32_bounded(10);
                    let rand_y = abs_y * 12 + random.next_i32_bounded(9);
                    let rand_z = abs_z * 16 + random.next_i32_bounded(10);

                    // Pack as i64 (BlockPos format)
                    let packed = pack_block_pos(rand_x, rand_y, rand_z);
                    packed_positions.push(packed);
                }
            }
        }

        Self {
            fluid_level_sampler,
            blocks,
            start_x,
            start_y,
            start_z,
            size_y,
            size_z: size_z as usize,
            levels: vec![None; total_size].into_boxed_slice(),
            packed_positions: packed_positions.into_boxed_slice(),
        }
    }

    /// Gets random positions for a given cell position.
    fn random_positions_for_pos(&self, x: i32, y: i32, z: i32) -> impl Iterator<Item = i64> + '_ {
        CHUNK_POS_OFFSETS.iter().filter_map(move |&(dx, dz)| {
            let cell_x = (x - self.start_x + i32::from(dx)) as usize;
            let cell_z = (z - self.start_z + i32::from(dz)) as usize;

            // Skip out of bounds
            if cell_x >= 4 || cell_z >= self.size_z {
                return None;
            }

            for dy in -1..=1 {
                let cell_y = (y - self.start_y + dy) as usize;
                if cell_y < self.size_y {
                    let index = packed_position_index(cell_x, cell_y, cell_z, self.size_y, self.size_z);
                    return Some(self.packed_positions[index]);
                }
            }
            None
        })
    }

    /// Gets or computes the water level at a position.
    fn get_water_level(
        &mut self,
        packed_pos: i64,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> FluidLevel {
        let x = unpack_x(packed_pos);
        let y = unpack_y(packed_pos);
        let z = unpack_z(packed_pos);

        let cell_x = (local_xz(x) - self.start_x) as usize;
        let cell_y = (local_y(y) - self.start_y) as usize;
        let cell_z = (local_xz(z) - self.start_z) as usize;

        if cell_x >= 4 || cell_z >= self.size_z || cell_y >= self.size_y {
            return self.get_fluid_level(x, y, z, router, sample_options, height_estimator);
        }

        let index = packed_position_index(cell_x, cell_y, cell_z, self.size_y, self.size_z);

        if let Some(ref level) = self.levels[index] {
            return level.clone();
        }

        let level = self.get_fluid_level(x, y, z, router, sample_options, height_estimator);
        self.levels[index] = Some(level.clone());
        level
    }

    /// Computes the fluid level at a position.
    fn get_fluid_level(
        &self,
        x: i32,
        y: i32,
        z: i32,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> FluidLevel {
        let default_level = self.fluid_level_sampler.get_fluid_level(x, y, z);
        let level_y = self.get_fluid_block_y(x, y, z, &default_level, router, sample_options, height_estimator);

        if level_y == MIN_HEIGHT_CELL {
            return default_level;
        }

        let block = self.get_fluid_block_state(x, y, z, &default_level, level_y, router, sample_options);
        FluidLevel::new(level_y, block)
    }

    /// Determines the Y level for fluid at a position.
    fn get_fluid_block_y(
        &self,
        x: i32,
        y: i32,
        z: i32,
        default_level: &FluidLevel,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> i32 {
        let surface_height = height_estimator.estimate_height(x, z);
        let pos = UnblendedNoisePos::new(x, y, z);

        // Check for deep dark (no fluid there)
        let erosion = router.erosion(&pos, sample_options);
        let depth = router.depth(&pos, sample_options);
        let is_deep_dark = erosion < -0.225 && depth > 0.9;

        let (d, e) = if is_deep_dark {
            (-1.0, -1.0)
        } else {
            let top_y = surface_height + 8 - y;
            let f = clamped_map(f64::from(top_y), 0.0, 64.0, 1.0, 0.0);

            let g = router.fluid_level_floodedness_noise(&pos, sample_options).clamp(-1.0, 1.0);
            let h = map(f, 1.0, 0.0, -0.3, 0.8);
            let k = map(f, 1.0, 0.0, -0.8, 0.4);

            (g - k, g - h)
        };

        if e > 0.0 {
            default_level.max_y_exclusive()
        } else if d > 0.0 {
            self.get_noise_based_fluid_level(x, y, z, surface_height, router, sample_options)
        } else {
            MIN_HEIGHT_CELL
        }
    }

    /// Gets a noise-based fluid level.
    fn get_noise_based_fluid_level(
        &self,
        x: i32,
        y: i32,
        z: i32,
        surface_height: i32,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> i32 {
        let grid_x = floor_div(x, 16);
        let grid_y = floor_div(y, 40);
        let grid_z = floor_div(z, 16);

        let local_y = grid_y * 40 + 20;

        let pos = UnblendedNoisePos::new(grid_x, grid_y, grid_z);
        let sample = router.fluid_level_spread_noise(&pos, sample_options) * 10.0;

        // Quantize to nearest multiple of 3
        let quantized = ((sample / 3.0).floor() as i32) * 3;
        let local_height = quantized + local_y;

        surface_height.min(local_height)
    }

    /// Determines if the fluid should be water or lava.
    fn get_fluid_block_state(
        &self,
        x: i32,
        y: i32,
        z: i32,
        default_level: &FluidLevel,
        level: i32,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> BlockStateId {
        // Deep aquifers (level <= -10) might be lava
        if level <= -10 && level != MIN_HEIGHT_CELL && default_level.block() != self.blocks.lava {
            let grid_x = floor_div(x, 64);
            let grid_y = floor_div(y, 40);
            let grid_z = floor_div(z, 64);

            let pos = UnblendedNoisePos::new(grid_x, grid_y, grid_z);
            let sample = router.lava_noise(&pos, sample_options);

            if sample.abs() > 0.3 {
                return self.blocks.lava;
            }
        }

        default_level.block()
    }

    /// Calculates the max distance weight: 1 - distance/25.
    #[inline]
    fn max_distance(dist1_sq: i32, dist2_sq: i32) -> f64 {
        let dist = (dist1_sq.max(dist2_sq) as f64).sqrt();
        1.0 - dist / 25.0
    }

    /// Calculates the density contribution between two fluid levels.
    fn calculate_density(
        barrier_sample: &mut Option<f64>,
        pos: &impl NoisePos,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        level_1: &FluidLevel,
        level_2: &FluidLevel,
        water: BlockStateId,
        lava: BlockStateId,
        air: BlockStateId,
    ) -> f64 {
        let y = pos.y();
        let block_state1 = level_1.get_block(y, air);
        let block_state2 = level_2.get_block(y, air);

        // If mixing water and lava, create barrier
        if (block_state1 == lava && block_state2 == water)
            || (block_state1 == water && block_state2 == lava)
        {
            return 2.0;
        }

        let level_diff = (level_1.max_y_exclusive() - level_2.max_y_exclusive()).abs();
        if level_diff == 0 {
            return 0.0;
        }

        let avg_level = 0.5 * (level_1.max_y_exclusive() + level_2.max_y_exclusive()) as f64;
        let scaled_level = y as f64 + 0.5 - avg_level;
        let halved_diff = level_diff as f64 / 2.0;

        let o = halved_diff - scaled_level.abs();
        let q = if scaled_level > 0.0 {
            if o > 0.0 { o / 1.5 } else { o / 2.5 }
        } else {
            let p = 3.0 + o;
            if p > 0.0 { p / 3.0 } else { p / 10.0 }
        };

        // Sample barrier noise if in interpolation range
        let r = if (-2.0..=2.0).contains(&q) {
            *barrier_sample.get_or_insert_with(|| router.barrier_noise(pos, sample_options))
        } else {
            0.0
        };

        2.0 * (r + q)
    }
}

impl AquiferSamplerImpl for WorldAquiferSampler {
    fn apply(
        &mut self,
        router: &mut ChunkNoiseRouter,
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> Option<BlockStateId> {
        let density = router.final_density(pos, sample_options);
        if density > 0.0 {
            return None; // Solid block
        }

        let sample_x = pos.x();
        let sample_y = pos.y();
        let sample_z = pos.z();

        // Find cell coordinates
        let scaled_x = local_xz(sample_x - 5);
        let scaled_y = local_y(sample_y + 1);
        let scaled_z = local_xz(sample_z - 5);

        // Find 3 nearest aquifer sample points
        let mut nearest: [(i64, i32); 3] = [(0, i32::MAX), (0, i32::MAX), (0, i32::MAX)];

        for packed_random in self.random_positions_for_pos(scaled_x, scaled_y, scaled_z) {
            let unpacked_x = unpack_x(packed_random);
            let unpacked_y = unpack_y(packed_random);
            let unpacked_z = unpack_z(packed_random);

            let dx = unpacked_x - sample_x;
            let dy = unpacked_y - sample_y;
            let dz = unpacked_z - sample_z;
            let dist_sq = dx * dx + dy * dy + dz * dz;

            // Insert into sorted array of 3 nearest
            if dist_sq < nearest[2].1 {
                if dist_sq < nearest[1].1 {
                    if dist_sq < nearest[0].1 {
                        nearest[2] = nearest[1];
                        nearest[1] = nearest[0];
                        nearest[0] = (packed_random, dist_sq);
                    } else {
                        nearest[2] = nearest[1];
                        nearest[1] = (packed_random, dist_sq);
                    }
                } else {
                    nearest[2] = (packed_random, dist_sq);
                }
            }
        }

        // Get fluid levels for nearest 3 points
        let level1 = self.get_water_level(nearest[0].0, router, sample_options, height_estimator);
        let level2 = self.get_water_level(nearest[1].0, router, sample_options, height_estimator);
        let level3 = self.get_water_level(nearest[2].0, router, sample_options, height_estimator);

        let dist1_sq = nearest[0].1;
        let dist2_sq = nearest[1].1;
        let dist3_sq = nearest[2].1;

        // Compute max_distance weight
        let d = Self::max_distance(dist1_sq, dist2_sq);

        // Determine block state from nearest level
        let block_state = level1.get_block(sample_y, self.blocks.air);

        // Check if completely submerged in one aquifer
        if d <= 0.0 {
            return Some(block_state);
        }

        // Calculate blended density between first two aquifers
        let mut barrier_sample = None;
        let e = d * Self::calculate_density(
            &mut barrier_sample,
            pos,
            router,
            sample_options,
            &level1,
            &level2,
            self.blocks.water,
            self.blocks.lava,
            self.blocks.air,
        );

        if density + e > 0.0 {
            return None; // Still solid
        }

        // Calculate blended density between first and third aquifers
        let f = Self::max_distance(dist1_sq, dist3_sq);
        if f > 0.0 {
            let g = d * f * Self::calculate_density(
                &mut barrier_sample,
                pos,
                router,
                sample_options,
                &level1,
                &level3,
                self.blocks.water,
                self.blocks.lava,
                self.blocks.air,
            );
            if density + g > 0.0 {
                return None;
            }
        }

        // Calculate blended density between second and third aquifers
        let g = Self::max_distance(dist2_sq, dist3_sq);
        if g > 0.0 {
            let h = d * g * Self::calculate_density(
                &mut barrier_sample,
                pos,
                router,
                sample_options,
                &level2,
                &level3,
                self.blocks.water,
                self.blocks.lava,
                self.blocks.air,
            );
            if density + h > 0.0 {
                return None;
            }
        }

        Some(block_state)
    }
}

// Block position packing/unpacking (matching BlockPos format)
const PACKED_X_BITS: u32 = 26;
const PACKED_Y_BITS: u32 = 12;
const PACKED_Z_BITS: u32 = 26;
const X_OFFSET: u32 = PACKED_Y_BITS + PACKED_Z_BITS; // 38
const Z_OFFSET: u32 = PACKED_Y_BITS; // 12
const PACKED_X_MASK: i64 = (1 << PACKED_X_BITS) - 1;
const PACKED_Y_MASK: i64 = (1 << PACKED_Y_BITS) - 1;
const PACKED_Z_MASK: i64 = (1 << PACKED_Z_BITS) - 1;

#[inline]
fn pack_block_pos(x: i32, y: i32, z: i32) -> i64 {
    ((i64::from(x) & PACKED_X_MASK) << X_OFFSET)
        | ((i64::from(z) & PACKED_Z_MASK) << Z_OFFSET)
        | (i64::from(y) & PACKED_Y_MASK)
}

#[inline]
fn unpack_x(packed: i64) -> i32 {
    let x = packed >> X_OFFSET;
    // Sign extend
    ((x << (64 - PACKED_X_BITS)) >> (64 - PACKED_X_BITS)) as i32
}

#[inline]
fn unpack_y(packed: i64) -> i32 {
    let y = packed & PACKED_Y_MASK;
    // Sign extend
    ((y << (64 - PACKED_Y_BITS)) >> (64 - PACKED_Y_BITS)) as i32
}

#[inline]
fn unpack_z(packed: i64) -> i32 {
    let z = (packed >> Z_OFFSET) & PACKED_Z_MASK;
    // Sign extend
    ((z << (64 - PACKED_Z_BITS)) >> (64 - PACKED_Z_BITS)) as i32
}
