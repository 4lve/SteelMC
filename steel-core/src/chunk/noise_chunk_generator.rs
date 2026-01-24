//! Noise-based chunk generator for vanilla-accurate terrain generation.
//!
//! This module implements terrain generation using the same density function system
//! as vanilla Minecraft, producing terrain that closely matches vanilla for the same seed.
//!
//! Key feature: Cell-based interpolation matching vanilla's 4x8x4 cell system.

// Uses coordinate variables (cell_x, cell_y, cell_z)
#![allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]

use steel_utils::{
    BlockStateId,
    density::{FunctionContext, MAX_Y, MIN_Y, NoiseRouter, SEA_LEVEL},
};

use crate::chunk::{chunk_access::ChunkAccess, chunk_generator::ChunkGenerator};

/// Default overworld noise settings matching vanilla Minecraft.
pub mod defaults {
    pub use steel_utils::density::{GLOBAL_OFFSET, MAX_Y, MIN_Y, SEA_LEVEL};
}

/// Cell dimensions for interpolation (matching vanilla).
const CELL_WIDTH: i32 = 4; // Horizontal cell size in blocks
const CELL_HEIGHT: i32 = 8; // Vertical cell size in blocks

/// Number of cells per chunk dimension.
const CELLS_X: usize = 16 / CELL_WIDTH as usize; // 4 cells
const CELLS_Z: usize = 16 / CELL_WIDTH as usize; // 4 cells

/// Cell homogeneity state for optimization.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CellState {
    /// All 8 corners are positive (solid).
    AllSolid,
    /// All 8 corners are negative or zero (air).
    AllAir,
    /// Mixed corners - need per-block interpolation.
    Mixed,
}

/// A chunk generator that uses 3D noise for vanilla-accurate terrain.
pub struct NoiseChunkGenerator {
    /// The noise router containing all terrain density functions.
    router: NoiseRouter,
    /// The world seed.
    seed: u64,
    /// Minimum Y for generation.
    min_y: i32,
    /// Maximum Y for generation.
    max_y: i32,
    /// Sea level Y coordinate.
    sea_level: i32,

    /// Block state ID for stone.
    pub stone: BlockStateId,
    /// Block state ID for water.
    pub water: BlockStateId,
    /// Block state ID for bedrock.
    pub bedrock: BlockStateId,
    /// Block state ID for deepslate.
    pub deepslate: BlockStateId,
}

impl NoiseChunkGenerator {
    /// Creates a new `NoiseChunkGenerator` with the given seed and block states.
    #[must_use]
    pub fn new(
        seed: u64,
        stone: BlockStateId,
        water: BlockStateId,
        bedrock: BlockStateId,
        deepslate: BlockStateId,
    ) -> Self {
        let router = NoiseRouter::overworld(seed);

        Self {
            router,
            seed,
            min_y: MIN_Y,
            max_y: MAX_Y,
            sea_level: SEA_LEVEL,
            stone,
            water,
            bedrock,
            deepslate,
        }
    }

    /// Get the world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Get the noise router.
    #[must_use]
    pub fn router(&self) -> &NoiseRouter {
        &self.router
    }

    /// Compute density at a world position using the full density function system.
    /// Negative values = air, positive values = solid.
    #[must_use]
    pub fn compute_density(&self, x: i32, y: i32, z: i32) -> f64 {
        let context = FunctionContext::new(x, y, z);
        self.router.final_density.compute(&context)
    }

    /// Determine if a position should be solid based on density.
    #[must_use]
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.compute_density(x, y, z) > 0.0
    }

    /// Number of vertical cells.
    fn cells_y(&self) -> usize {
        ((self.max_y - self.min_y) / CELL_HEIGHT) as usize
    }

    /// Sample density at all cell corners for a chunk.
    /// Returns a 3D array indexed by `[cell_x][cell_z][cell_y]` with corner values.
    /// Each cell needs 8 corners, but adjacent cells share corners, so we store
    /// `(CELLS_X+1) * (CELLS_Z+1) * (cells_y+1)` corner values.
    fn sample_cell_corners(&self, base_x: i32, base_z: i32) -> Vec<f64> {
        let cells_y = self.cells_y();
        let corners_x = CELLS_X + 1; // 5
        let corners_z = CELLS_Z + 1; // 5
        let corners_y = cells_y + 1; // 49

        let mut corners = vec![0.0; corners_x * corners_z * corners_y];

        for cx in 0..corners_x {
            for cz in 0..corners_z {
                for cy in 0..corners_y {
                    let world_x = base_x + (cx as i32) * CELL_WIDTH;
                    let world_y = self.min_y + (cy as i32) * CELL_HEIGHT;
                    let world_z = base_z + (cz as i32) * CELL_WIDTH;

                    let idx = cx * corners_z * corners_y + cz * corners_y + cy;
                    corners[idx] = self.compute_density(world_x, world_y, world_z);
                }
            }
        }

        corners
    }

    /// Get corner value from the corners array.
    #[inline]
    fn get_corner(&self, corners: &[f64], cx: usize, cz: usize, cy: usize) -> f64 {
        let corners_z = CELLS_Z + 1;
        let corners_y = self.cells_y() + 1;
        corners[cx * corners_z * corners_y + cz * corners_y + cy]
    }

    /// Trilinear interpolation between 8 corner values.
    #[inline]
    fn trilinear_interpolate(
        &self,
        corners: &[f64],
        cell_x: usize,
        cell_z: usize,
        cell_y: usize,
        dx: f64,
        dy: f64,
        dz: f64,
    ) -> f64 {
        // Get 8 corners of this cell
        let c000 = self.get_corner(corners, cell_x, cell_z, cell_y);
        let c001 = self.get_corner(corners, cell_x, cell_z, cell_y + 1);
        let c010 = self.get_corner(corners, cell_x, cell_z + 1, cell_y);
        let c011 = self.get_corner(corners, cell_x, cell_z + 1, cell_y + 1);
        let c100 = self.get_corner(corners, cell_x + 1, cell_z, cell_y);
        let c101 = self.get_corner(corners, cell_x + 1, cell_z, cell_y + 1);
        let c110 = self.get_corner(corners, cell_x + 1, cell_z + 1, cell_y);
        let c111 = self.get_corner(corners, cell_x + 1, cell_z + 1, cell_y + 1);

        // Interpolate along Y first
        let c00 = lerp(dy, c000, c001);
        let c01 = lerp(dy, c010, c011);
        let c10 = lerp(dy, c100, c101);
        let c11 = lerp(dy, c110, c111);

        // Then along Z
        let c0 = lerp(dz, c00, c01);
        let c1 = lerp(dz, c10, c11);

        // Finally along X
        lerp(dx, c0, c1)
    }

    /// Get interpolated density at a block position within the chunk.
    #[inline]
    fn get_interpolated_density(
        &self,
        corners: &[f64],
        local_x: i32,
        local_y: i32,
        local_z: i32,
    ) -> f64 {
        // Determine which cell this block is in
        let cell_x = (local_x / CELL_WIDTH) as usize;
        let cell_z = (local_z / CELL_WIDTH) as usize;
        let cell_y = (local_y / CELL_HEIGHT) as usize;

        // Position within the cell (0.0 to 1.0)
        let dx = f64::from(local_x % CELL_WIDTH) / f64::from(CELL_WIDTH);
        let dz = f64::from(local_z % CELL_WIDTH) / f64::from(CELL_WIDTH);
        let dy = f64::from(local_y % CELL_HEIGHT) / f64::from(CELL_HEIGHT);

        self.trilinear_interpolate(corners, cell_x, cell_z, cell_y, dx, dy, dz)
    }

    /// Determine the block to place at a given position.
    /// Returns None for air (to skip placement).
    #[inline]
    fn determine_block(
        &self,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        is_solid: bool,
        bedrock_top: i32,
    ) -> Option<BlockStateId> {
        if world_y == self.min_y {
            // Bottom layer is always bedrock
            Some(self.bedrock)
        } else if world_y < bedrock_top {
            // Bedrock layer with random gaps
            let hash = position_hash(world_x, world_y, world_z);
            let bedrock_chance = (bedrock_top - world_y) as u32;
            if hash % 5 < bedrock_chance {
                Some(self.bedrock)
            } else if is_solid {
                if world_y < 0 {
                    Some(self.deepslate)
                } else {
                    Some(self.stone)
                }
            } else if world_y <= self.sea_level {
                Some(self.water)
            } else {
                None // Air
            }
        } else if is_solid {
            // Solid terrain
            if world_y < 0 {
                Some(self.deepslate)
            } else {
                Some(self.stone)
            }
        } else if world_y <= self.sea_level {
            // Below sea level - water
            Some(self.water)
        } else {
            // Air - skip
            None
        }
    }
}

/// Linear interpolation.
#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

impl ChunkGenerator for NoiseChunkGenerator {
    fn create_structures(&self, _chunk: &ChunkAccess) {
        // TODO: Structure generation
    }

    fn create_biomes(&self, _chunk: &ChunkAccess) {
        // TODO: Biome generation
    }

    fn fill_from_noise(&self, chunk: &ChunkAccess) {
        let chunk_pos = chunk.pos();
        let base_x = chunk_pos.0.x * 16;
        let base_z = chunk_pos.0.y * 16; // ChunkPos uses Vector2(x, z) where z is stored in y

        let cells_y = self.cells_y();

        // Sample density at cell corners (much fewer samples than per-block)
        let corners = self.sample_cell_corners(base_x, base_z);

        // Pre-compute cell homogeneity to enable fast-path filling
        // For each cell, check if all 8 corners have the same sign
        let mut cell_homogeneous = vec![CellState::Mixed; CELLS_X * CELLS_Z * cells_y];

        for cell_x in 0..CELLS_X {
            for cell_z in 0..CELLS_Z {
                for cell_y in 0..cells_y {
                    let c000 = self.get_corner(&corners, cell_x, cell_z, cell_y);
                    let c001 = self.get_corner(&corners, cell_x, cell_z, cell_y + 1);
                    let c010 = self.get_corner(&corners, cell_x, cell_z + 1, cell_y);
                    let c011 = self.get_corner(&corners, cell_x, cell_z + 1, cell_y + 1);
                    let c100 = self.get_corner(&corners, cell_x + 1, cell_z, cell_y);
                    let c101 = self.get_corner(&corners, cell_x + 1, cell_z, cell_y + 1);
                    let c110 = self.get_corner(&corners, cell_x + 1, cell_z + 1, cell_y);
                    let c111 = self.get_corner(&corners, cell_x + 1, cell_z + 1, cell_y + 1);

                    let all_positive = c000 > 0.0
                        && c001 > 0.0
                        && c010 > 0.0
                        && c011 > 0.0
                        && c100 > 0.0
                        && c101 > 0.0
                        && c110 > 0.0
                        && c111 > 0.0;
                    let all_negative = c000 <= 0.0
                        && c001 <= 0.0
                        && c010 <= 0.0
                        && c011 <= 0.0
                        && c100 <= 0.0
                        && c101 <= 0.0
                        && c110 <= 0.0
                        && c111 <= 0.0;

                    let idx = cell_x * CELLS_Z * cells_y + cell_z * cells_y + cell_y;
                    cell_homogeneous[idx] = if all_positive {
                        CellState::AllSolid
                    } else if all_negative {
                        CellState::AllAir
                    } else {
                        CellState::Mixed
                    };
                }
            }
        }

        // Fill chunk with terrain using cell-based optimization
        let bedrock_top = self.min_y + 5;

        for cell_x in 0..CELLS_X {
            for cell_z in 0..CELLS_Z {
                let local_x_start = (cell_x as i32) * CELL_WIDTH;
                let local_z_start = (cell_z as i32) * CELL_WIDTH;

                for cell_y in 0..cells_y {
                    let local_y_start = (cell_y as i32) * CELL_HEIGHT;
                    let cell_idx = cell_x * CELLS_Z * cells_y + cell_z * cells_y + cell_y;
                    let cell_state = cell_homogeneous[cell_idx];

                    if cell_state == CellState::Mixed {
                        // Mixed cell: need per-block interpolation
                        for dx in 0..CELL_WIDTH {
                            for dz in 0..CELL_WIDTH {
                                let local_x = local_x_start + dx;
                                let local_z = local_z_start + dz;
                                let world_x = base_x + local_x;
                                let world_z = base_z + local_z;

                                for dy in 0..CELL_HEIGHT {
                                    let local_y = (local_y_start + dy) as usize;
                                    let world_y = self.min_y + local_y as i32;

                                    // Get interpolated density
                                    let density = self.get_interpolated_density(
                                        &corners,
                                        local_x,
                                        local_y as i32,
                                        local_z,
                                    );
                                    let is_solid = density > 0.0;

                                    let block = self.determine_block(
                                        world_x,
                                        world_y,
                                        world_z,
                                        is_solid,
                                        bedrock_top,
                                    );

                                    if let Some(block) = block {
                                        chunk.set_relative_block(
                                            local_x as usize,
                                            local_y,
                                            local_z as usize,
                                            block,
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // Fast path: entire cell is homogeneous
                        let is_solid = cell_state == CellState::AllSolid;

                        for dx in 0..CELL_WIDTH {
                            for dz in 0..CELL_WIDTH {
                                let local_x = local_x_start + dx;
                                let local_z = local_z_start + dz;
                                let world_x = base_x + local_x;
                                let world_z = base_z + local_z;

                                for dy in 0..CELL_HEIGHT {
                                    let local_y = (local_y_start + dy) as usize;
                                    let world_y = self.min_y + local_y as i32;

                                    let block = self.determine_block(
                                        world_x,
                                        world_y,
                                        world_z,
                                        is_solid,
                                        bedrock_top,
                                    );

                                    if let Some(block) = block {
                                        chunk.set_relative_block(
                                            local_x as usize,
                                            local_y,
                                            local_z as usize,
                                            block,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_surface(&self, _chunk: &ChunkAccess) {
        // TODO: Surface generation (grass, sand, etc.)
        // This would replace top stone/deepslate with appropriate surface blocks
    }

    fn apply_carvers(&self, _chunk: &ChunkAccess) {
        // TODO: Cave carvers
    }

    fn apply_biome_decorations(&self, _chunk: &ChunkAccess) {
        // TODO: Trees, ores, etc.
    }
}

/// Simple position-based hash for bedrock variation.
fn position_hash(x: i32, y: i32, z: i32) -> u32 {
    let mut h = x.wrapping_mul(3_129_871) as u32;
    h ^= z.wrapping_mul(116_129_781) as u32;
    h ^= y as u32;
    h = h
        .wrapping_mul(h)
        .wrapping_mul(42_317_861)
        .wrapping_add(h.wrapping_mul(11));
    h >> 16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let gen1 = NoiseChunkGenerator::new(
            12345,
            BlockStateId(1),
            BlockStateId(2),
            BlockStateId(3),
            BlockStateId(4),
        );
        let gen2 = NoiseChunkGenerator::new(
            12345,
            BlockStateId(1),
            BlockStateId(2),
            BlockStateId(3),
            BlockStateId(4),
        );

        // Same seed should produce same density
        assert_eq!(
            gen1.compute_density(100, 64, 100).to_bits(),
            gen2.compute_density(100, 64, 100).to_bits()
        );
    }

    #[test]
    fn test_terrain_shape() {
        let generator = NoiseChunkGenerator::new(
            42,
            BlockStateId(1),
            BlockStateId(2),
            BlockStateId(3),
            BlockStateId(4),
        );

        // Very deep underground should almost always be solid
        let mut solid_count = 0;
        for x in 0..10 {
            for z in 0..10 {
                if generator.is_solid(x * 100, -50, z * 100) {
                    solid_count += 1;
                }
            }
        }
        assert!(solid_count > 80, "Deep underground should be mostly solid");

        // Very high up should almost always be air
        let mut air_count = 0;
        for x in 0..10 {
            for z in 0..10 {
                if !generator.is_solid(x * 100, 200, z * 100) {
                    air_count += 1;
                }
            }
        }
        assert!(air_count > 80, "High altitude should be mostly air");
    }

    #[test]
    fn test_interpolation_at_corners() {
        let generator = NoiseChunkGenerator::new(
            42,
            BlockStateId(1),
            BlockStateId(2),
            BlockStateId(3),
            BlockStateId(4),
        );

        // At cell corners, interpolated value should equal direct computation
        let corners = generator.sample_cell_corners(0, 0);

        // Check corner at (0, 0, 0) in local coords
        let interpolated = generator.get_interpolated_density(&corners, 0, 0, 0);
        let direct = generator.compute_density(0, generator.min_y, 0);
        assert!(
            (interpolated - direct).abs() < 1e-10,
            "At corner: interpolated {interpolated} != direct {direct}"
        );

        // Check corner at (4, 8, 4) - next cell corner
        let interpolated = generator.get_interpolated_density(&corners, 4, 8, 4);
        let direct = generator.compute_density(4, generator.min_y + 8, 4);
        assert!(
            (interpolated - direct).abs() < 1e-10,
            "At corner: interpolated {interpolated} != direct {direct}"
        );
    }
}
