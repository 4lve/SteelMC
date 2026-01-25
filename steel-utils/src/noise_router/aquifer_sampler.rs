//! Aquifer sampler for cave and underground water/lava generation.
//!
//! This module contains the aquifer sampling system that determines where
//! water and lava appear in caves and underground areas.

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

/// Minimum Y value marker (way below min Y).
/// This must be a reasonable value that won't cause integer overflow in barrier calculations.
/// Matches vanilla's DimensionType.MIN_HEIGHT * 16 = -2032 * 16 = -32512
const WAY_BELOW_MIN_Y: i32 = -32512;

/// 13 chunk position offsets for surface height sampling.
/// Order matches vanilla - (0,0) MUST be first for early-return logic to work correctly.
const SURFACE_SAMPLING_OFFSETS_IN_CHUNKS: [(i8, i8); 13] = [
    (0, 0),   // Must be first - checked for early returns
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (1, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Converts section coordinate to block coordinate.
#[inline]
fn section_to_block(section: i32) -> i32 {
    section << 4
}

/// Converts block coordinate to grid X coordinate (16-block spacing).
#[inline]
fn grid_x(block_coord: i32) -> i32 {
    block_coord >> 4
}

/// Converts grid X coordinate back to block coordinate with offset.
#[inline]
fn from_grid_x(grid_coord: i32, offset: i32) -> i32 {
    (grid_coord << 4) + offset
}

/// Converts block coordinate to grid Y coordinate (12-block spacing).
#[inline]
fn grid_y(block_coord: i32) -> i32 {
    floor_div(block_coord, 12)
}

/// Converts grid Y coordinate back to block coordinate with offset.
#[inline]
fn from_grid_y(grid_coord: i32, offset: i32) -> i32 {
    grid_coord * 12 + offset
}

/// Converts block coordinate to grid Z coordinate (16-block spacing).
#[inline]
fn grid_z(block_coord: i32) -> i32 {
    block_coord >> 4
}

/// Converts grid Z coordinate back to block coordinate with offset.
#[inline]
fn from_grid_z(grid_coord: i32, offset: i32) -> i32 {
    (grid_coord << 4) + offset
}

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

/// Full world aquifer sampler with underground water/lava pockets.
///
/// This implementation matches vanilla Minecraft's NoiseBasedAquifer.
pub struct WorldAquiferSampler {
    /// Fluid level sampler for default levels.
    fluid_level_sampler: FluidLevelSampler,
    /// Block states for water/lava/air.
    blocks: AquiferBlocks,
    /// Random deriver for position-based random.
    random_deriver: RandomSplitter,
    /// Minimum grid X coordinate.
    min_grid_x: i32,
    /// Minimum grid Y coordinate.
    min_grid_y: i32,
    /// Minimum grid Z coordinate.
    min_grid_z: i32,
    /// Grid size in X dimension.
    grid_size_x: usize,
    /// Grid size in Z dimension.
    grid_size_z: usize,
    /// Cached aquifer fluid levels (lazily computed).
    aquifer_cache: Box<[Option<FluidLevel>]>,
    /// Cached random positions (lazily computed, Long.MAX_VALUE = not computed).
    aquifer_location_cache: Box<[i64]>,
}

impl WorldAquiferSampler {
    /// Creates a new world aquifer sampler.
    #[must_use]
    pub fn new(
        chunk_x: i32,
        chunk_z: i32,
        random_deriver: RandomSplitter,
        minimum_y: i8,
        height: u16,
        fluid_level_sampler: FluidLevelSampler,
        blocks: AquiferBlocks,
    ) -> Self {
        let chunk_min_x = chunk_x * 16;
        let chunk_max_x = chunk_min_x + 15;
        let chunk_min_z = chunk_z * 16;
        let chunk_max_z = chunk_min_z + 15;

        // Calculate grid bounds with SAMPLE_OFFSET_X = -5, SAMPLE_OFFSET_Z = -5
        let min_grid_x = grid_x(chunk_min_x - 5);
        let max_grid_x = grid_x(chunk_max_x - 5) + 1;
        let grid_size_x = (max_grid_x - min_grid_x + 1) as usize;

        // SAMPLE_OFFSET_Y = 1, MIN_CELL_SAMPLE_Y = -1, MAX_CELL_SAMPLE_Y = 1
        let min_grid_y = grid_y(i32::from(minimum_y) + 1) - 1;
        let max_grid_y = grid_y(i32::from(minimum_y) + i32::from(height) + 1) + 1;
        let grid_size_y = (max_grid_y - min_grid_y + 1) as usize;

        let min_grid_z = grid_z(chunk_min_z - 5);
        let max_grid_z = grid_z(chunk_max_z - 5) + 1;
        let grid_size_z = (max_grid_z - min_grid_z + 1) as usize;

        let cache_size = grid_size_x * grid_size_y * grid_size_z;

        Self {
            fluid_level_sampler,
            blocks,
            random_deriver,
            min_grid_x,
            min_grid_y,
            min_grid_z,
            grid_size_x,
            grid_size_z,
            aquifer_cache: vec![None; cache_size].into_boxed_slice(),
            aquifer_location_cache: vec![i64::MAX; cache_size].into_boxed_slice(),
        }
    }

    /// Calculates cache index for grid coordinates.
    /// Formula: (y * gridSizeZ + z) * gridSizeX + x (vanilla order)
    #[inline]
    fn get_index(&self, grid_x: i32, grid_y: i32, grid_z: i32) -> usize {
        let x = (grid_x - self.min_grid_x) as usize;
        let y = (grid_y - self.min_grid_y) as usize;
        let z = (grid_z - self.min_grid_z) as usize;
        (y * self.grid_size_z + z) * self.grid_size_x + x
    }

    /// Computes similarity between two squared distances.
    /// Returns 1.0 - (dist2 - dist1) / 25.0
    #[inline]
    fn similarity(dist_sq_1: i32, dist_sq_2: i32) -> f64 {
        1.0 - f64::from(dist_sq_2 - dist_sq_1) / 25.0
    }

    /// Gets the aquifer status (fluid level) at a cache index.
    fn get_aquifer_status(
        &mut self,
        index: usize,
        router: &mut ChunkNoiseRouter,
        height_estimator: &mut SurfaceHeightEstimateSampler,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> FluidLevel {
        if let Some(ref level) = self.aquifer_cache[index] {
            return level.clone();
        }

        let location = self.aquifer_location_cache[index];
        let x = unpack_x(location);
        let y = unpack_y(location);
        let z = unpack_z(location);

        let level = self.compute_fluid(x, y, z, router, height_estimator, sample_options);
        self.aquifer_cache[index] = Some(level.clone());
        level
    }

    /// Computes the fluid level at a position using 13-point surface height sampling.
    fn compute_fluid(
        &self,
        x: i32,
        y: i32,
        z: i32,
        router: &mut ChunkNoiseRouter,
        height_estimator: &mut SurfaceHeightEstimateSampler,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> FluidLevel {
        let global_fluid = self.fluid_level_sampler.get_fluid_level(x, y, z);
        let mut lowest_preliminary_surface = i32::MAX;
        let top_of_aquifer_cell = y + 12;
        let bottom_of_aquifer_cell = y - 12;
        let mut surface_at_center_is_under_global_fluid_level = false;

        for (offset_x, offset_z) in SURFACE_SAMPLING_OFFSETS_IN_CHUNKS {
            let sample_x = x + section_to_block(i32::from(offset_x));
            let sample_z = z + section_to_block(i32::from(offset_z));

            let preliminary_surface_level = height_estimator.estimate_height(sample_x, sample_z);
            let adjusted_surface_level = preliminary_surface_level + 8;
            let is_start = offset_x == 0 && offset_z == 0;

            if is_start && bottom_of_aquifer_cell > adjusted_surface_level {
                return global_fluid;
            }

            let top_pokes_above_surface = top_of_aquifer_cell > adjusted_surface_level;
            if top_pokes_above_surface || is_start {
                let global_fluid_at_surface = self.fluid_level_sampler.get_fluid_level(sample_x, adjusted_surface_level, sample_z);
                if global_fluid_at_surface.get_block(adjusted_surface_level, self.blocks.air) != self.blocks.air {
                    if is_start {
                        surface_at_center_is_under_global_fluid_level = true;
                    }
                    if top_pokes_above_surface {
                        return global_fluid_at_surface;
                    }
                }
            }

            lowest_preliminary_surface = lowest_preliminary_surface.min(preliminary_surface_level);
        }

        let fluid_surface_level = self.compute_surface_level(
            x, y, z,
            &global_fluid,
            lowest_preliminary_surface,
            surface_at_center_is_under_global_fluid_level,
            router,
            sample_options,
        );

        FluidLevel::new(
            fluid_surface_level,
            self.compute_fluid_type(x, y, z, &global_fluid, fluid_surface_level, router, sample_options),
        )
    }

    /// Computes the surface level for fluid at a position.
    fn compute_surface_level(
        &self,
        x: i32,
        y: i32,
        z: i32,
        global_fluid: &FluidLevel,
        lowest_preliminary_surface: i32,
        surface_at_center_is_under_global_fluid_level: bool,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> i32 {
        let pos = UnblendedNoisePos::new(x, y, z);

        // Check for deep dark region (no aquifers there)
        let erosion = router.erosion(&pos, sample_options);
        let depth = router.depth(&pos, sample_options);
        let is_deep_dark = erosion < -0.225 && depth > 0.9;

        let (partially_flooded, fully_flooded) = if is_deep_dark {
            (-1.0, -1.0)
        } else {
            let distance_below_surface = lowest_preliminary_surface + 8 - y;
            let floodedness_factor = if surface_at_center_is_under_global_fluid_level {
                clamped_map(f64::from(distance_below_surface), 0.0, 64.0, 1.0, 0.0)
            } else {
                0.0
            };

            let floodedness_noise = router.fluid_level_floodedness_noise(&pos, sample_options).clamp(-1.0, 1.0);
            let fully_flooded_threshold = map(floodedness_factor, 1.0, 0.0, -0.3, 0.8);
            let partially_flooded_threshold = map(floodedness_factor, 1.0, 0.0, -0.8, 0.4);

            (floodedness_noise - partially_flooded_threshold, floodedness_noise - fully_flooded_threshold)
        };

        if fully_flooded > 0.0 {
            global_fluid.max_y_exclusive()
        } else if partially_flooded > 0.0 {
            self.compute_randomized_fluid_surface_level(x, y, z, lowest_preliminary_surface, router, sample_options)
        } else {
            WAY_BELOW_MIN_Y
        }
    }

    /// Computes a randomized fluid surface level.
    fn compute_randomized_fluid_surface_level(
        &self,
        x: i32,
        y: i32,
        z: i32,
        lowest_preliminary_surface: i32,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> i32 {
        let fluid_level_cell_x = floor_div(x, 16);
        let fluid_level_cell_y = floor_div(y, 40);
        let fluid_level_cell_z = floor_div(z, 16);

        let fluid_cell_middle_y = fluid_level_cell_y * 40 + 20;

        let pos = UnblendedNoisePos::new(fluid_level_cell_x, fluid_level_cell_y, fluid_level_cell_z);
        let fluid_level_spread = router.fluid_level_spread_noise(&pos, sample_options) * 10.0;
        let fluid_level_spread_quantized = ((fluid_level_spread / 3.0).floor() as i32) * 3;
        let target_fluid_surface_level = fluid_cell_middle_y + fluid_level_spread_quantized;

        lowest_preliminary_surface.min(target_fluid_surface_level)
    }

    /// Determines the fluid type (water or lava) at a position.
    fn compute_fluid_type(
        &self,
        x: i32,
        y: i32,
        z: i32,
        global_fluid: &FluidLevel,
        fluid_surface_level: i32,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> BlockStateId {
        if fluid_surface_level <= -10 && fluid_surface_level != WAY_BELOW_MIN_Y && global_fluid.block() != self.blocks.lava {
            let fluid_type_cell_x = floor_div(x, 64);
            let fluid_type_cell_y = floor_div(y, 40);
            let fluid_type_cell_z = floor_div(z, 64);

            let pos = UnblendedNoisePos::new(fluid_type_cell_x, fluid_type_cell_y, fluid_type_cell_z);
            let lava_noise = router.lava_noise(&pos, sample_options);

            if lava_noise.abs() > 0.3 {
                return self.blocks.lava;
            }
        }

        global_fluid.block()
    }

    /// Calculates the pressure/density contribution between two fluid levels.
    fn calculate_pressure(
        &self,
        pos: &impl NoisePos,
        barrier_noise_value: &mut Option<f64>,
        router: &mut ChunkNoiseRouter,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        status1: &FluidLevel,
        status2: &FluidLevel,
    ) -> f64 {
        let y = pos.y();
        let type1 = status1.get_block(y, self.blocks.air);
        let type2 = status2.get_block(y, self.blocks.air);

        // Water/lava mixing creates barrier
        if (type1 == self.blocks.lava && type2 == self.blocks.water)
            || (type1 == self.blocks.water && type2 == self.blocks.lava)
        {
            return 2.0;
        }

        let fluid_y_diff = (status1.max_y_exclusive() - status2.max_y_exclusive()).abs();
        if fluid_y_diff == 0 {
            return 0.0;
        }

        let average_fluid_y = 0.5 * f64::from(status1.max_y_exclusive() + status2.max_y_exclusive());
        let how_far_above_average = f64::from(y) + 0.5 - average_fluid_y;
        let base_value = f64::from(fluid_y_diff) / 2.0;

        let distance_from_barrier_edge = base_value - how_far_above_average.abs();

        let gradient = if how_far_above_average > 0.0 {
            let center_point = distance_from_barrier_edge;
            if center_point > 0.0 {
                center_point / 1.5
            } else {
                center_point / 2.5
            }
        } else {
            let center_point = 3.0 + distance_from_barrier_edge;
            if center_point > 0.0 {
                center_point / 3.0
            } else {
                center_point / 10.0
            }
        };

        let noise_value = if !(-2.0..=2.0).contains(&gradient) {
            0.0
        } else {
            *barrier_noise_value.get_or_insert_with(|| router.barrier_noise(pos, sample_options))
        };

        2.0 * (noise_value + gradient)
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

        let pos_x = pos.x();
        let pos_y = pos.y();
        let pos_z = pos.z();

        // Check global fluid first
        let global_fluid = self.fluid_level_sampler.get_fluid_level(pos_x, pos_y, pos_z);

        if global_fluid.get_block(pos_y, self.blocks.air) == self.blocks.lava {
            return Some(self.blocks.lava);
        }

        // Compute anchor grid coordinates with sample offsets
        let x_anchor = grid_x(pos_x - 5);
        let y_anchor = grid_y(pos_y + 1);
        let z_anchor = grid_z(pos_z - 5);

        // Track 4 closest aquifer points
        let mut dist_sq_1 = i32::MAX;
        let mut dist_sq_2 = i32::MAX;
        let mut dist_sq_3 = i32::MAX;
        let mut closest_index_1 = 0usize;
        let mut closest_index_2 = 0usize;
        let mut closest_index_3 = 0usize;

        // Iterate over 2x3x2 grid of cells in same order as vanilla
        // Pumpkin's order: y goes 1,0,-1; for each y, (x,z) goes (1,1),(1,0),(0,1),(0,0)
        for y1 in (-1..=1).rev() {
            for x1 in (0..=1).rev() {
                for z1 in (0..=1).rev() {
                    let spaced_grid_x = x_anchor + x1;
                    let spaced_grid_y = y_anchor + y1;
                    let spaced_grid_z = z_anchor + z1;

                    let index = self.get_index(spaced_grid_x, spaced_grid_y, spaced_grid_z);

                    // Get or compute random position for this cell
                    let location = if self.aquifer_location_cache[index] != i64::MAX {
                        self.aquifer_location_cache[index]
                    } else {
                        let mut random = self.random_deriver.at(spaced_grid_x, spaced_grid_y, spaced_grid_z);
                        let loc = pack_block_pos(
                            from_grid_x(spaced_grid_x, random.next_i32_bounded(10)),
                            from_grid_y(spaced_grid_y, random.next_i32_bounded(9)),
                            from_grid_z(spaced_grid_z, random.next_i32_bounded(10)),
                        );
                        self.aquifer_location_cache[index] = loc;
                        loc
                    };

                    let dx = unpack_x(location) - pos_x;
                    let dy = unpack_y(location) - pos_y;
                    let dz = unpack_z(location) - pos_z;
                    let new_dist = dx * dx + dy * dy + dz * dz;

                    // Insertion sort into 3 closest
                    // Use strict > to match vanilla tie-breaking (earlier positions win)
                    if dist_sq_1 > new_dist {
                        closest_index_3 = closest_index_2;
                        closest_index_2 = closest_index_1;
                        closest_index_1 = index;
                        dist_sq_3 = dist_sq_2;
                        dist_sq_2 = dist_sq_1;
                        dist_sq_1 = new_dist;
                    } else if dist_sq_2 > new_dist {
                        closest_index_3 = closest_index_2;
                        closest_index_2 = index;
                        dist_sq_3 = dist_sq_2;
                        dist_sq_2 = new_dist;
                    } else if dist_sq_3 > new_dist {
                        closest_index_3 = index;
                        dist_sq_3 = new_dist;
                    }
                }
            }
        }

        // Get fluid status for closest point
        let status1 = self.get_aquifer_status(closest_index_1, router, height_estimator, sample_options);
        let similarity_12 = Self::similarity(dist_sq_1, dist_sq_2);
        let fluid_state = status1.get_block(pos_y, self.blocks.air);

        if similarity_12 <= 0.0 {
            return Some(fluid_state);
        }

        // Water above lava transition
        if fluid_state == self.blocks.water
            && self.fluid_level_sampler
                .get_fluid_level(pos_x, pos_y - 1, pos_z)
                .get_block(pos_y - 1, self.blocks.air) == self.blocks.lava
        {
            return Some(fluid_state);
        }

        // Calculate barrier between first two aquifers
        let mut barrier_noise_value = None;
        let status2 = self.get_aquifer_status(closest_index_2, router, height_estimator, sample_options);
        let barrier_12 = similarity_12 * self.calculate_pressure(
            pos,
            &mut barrier_noise_value,
            router,
            sample_options,
            &status1,
            &status2,
        );

        if density + barrier_12 > 0.0 {
            return None; // Still solid
        }

        // Check third aquifer
        let status3 = self.get_aquifer_status(closest_index_3, router, height_estimator, sample_options);
        let similarity_13 = Self::similarity(dist_sq_1, dist_sq_3);
        if similarity_13 > 0.0 {
            let barrier_13 = similarity_12 * similarity_13 * self.calculate_pressure(
                pos,
                &mut barrier_noise_value,
                router,
                sample_options,
                &status1,
                &status3,
            );
            if density + barrier_13 > 0.0 {
                return None;
            }
        }

        let similarity_23 = Self::similarity(dist_sq_2, dist_sq_3);
        if similarity_23 > 0.0 {
            let barrier_23 = similarity_12 * similarity_23 * self.calculate_pressure(
                pos,
                &mut barrier_noise_value,
                router,
                sample_options,
                &status2,
                &status3,
            );
            if density + barrier_23 > 0.0 {
                return None;
            }
        }

        Some(fluid_state)
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
