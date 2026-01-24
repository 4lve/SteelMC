//! Pumpkin-style noise chunk generator using the data-driven noise router.
//!
//! This generator uses the same density function component stack as Pumpkin/vanilla
//! for accurate terrain generation with proper cell-based interpolation.

// Uses coordinate variables (cell_x, cell_y, cell_z, start_cell_z, etc.)
#![allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]

use steel_utils::{
    BlockStateId,
    noise_router::{
        OVERWORLD_BASE_NOISE_ROUTER,
        chunk_density_function::{
            ChunkNoiseFunctionBuilderOptions, ChunkNoiseFunctionSampleOptions, SampleAction,
            WrapperData,
        },
        chunk_noise_router::ChunkNoiseRouter,
        density_function::{IndexToNoisePos, NoisePos, UnblendedNoisePos},
        proto_noise_router::ProtoNoiseRouters,
    },
};

use crate::chunk::{chunk_access::ChunkAccess, chunk_generator::ChunkGenerator};

/// Floor division matching vanilla Minecraft.
#[inline]
fn floor_div(a: i32, b: i32) -> i32 {
    let q = a / b;
    let r = a % b;
    if r != 0 && (a < 0) != (b < 0) {
        q - 1
    } else {
        q
    }
}

/// Floor modulo matching vanilla Minecraft.
#[inline]
fn floor_mod(a: usize, b: usize) -> usize {
    ((a % b) + b) % b
}

/// Cell dimensions for interpolation (matching vanilla).
const CELL_WIDTH: i32 = 4;
const CELL_HEIGHT: i32 = 8;

/// Minimum Y for overworld.
const MIN_Y: i32 = -64;
/// Maximum Y for overworld.
const MAX_Y: i32 = 320;
/// Sea level for overworld.
const SEA_LEVEL: i32 = 63;

/// Generation shape configuration.
struct GenerationShape {
    min_y: i32,
    height: i32,
    horizontal_cell_block_count: i32,
    vertical_cell_block_count: i32,
}

impl GenerationShape {
    fn overworld() -> Self {
        Self {
            min_y: MIN_Y,
            height: MAX_Y - MIN_Y,
            horizontal_cell_block_count: CELL_WIDTH,
            vertical_cell_block_count: CELL_HEIGHT,
        }
    }
}

/// Random configuration for world generation.
pub struct RandomConfig {
    /// The world seed for random generation.
    pub seed: u64,
}

impl RandomConfig {
    /// Creates a new random configuration with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
}

/// A chunk generator using the Pumpkin-style noise router with proper interpolation.
pub struct PumpkinNoiseGenerator {
    /// Proto noise routers (built once from seed).
    proto_routers: ProtoNoiseRouters,
    /// Generation shape.
    shape: GenerationShape,
    /// The world seed.
    seed: u64,

    /// Block state ID for stone.
    pub stone: BlockStateId,
    /// Block state ID for water.
    pub water: BlockStateId,
    /// Block state ID for bedrock.
    pub bedrock: BlockStateId,
    /// Block state ID for deepslate.
    pub deepslate: BlockStateId,
}

impl PumpkinNoiseGenerator {
    /// Creates a new generator with the given seed and block states.
    #[must_use]
    pub fn new(
        seed: u64,
        stone: BlockStateId,
        water: BlockStateId,
        bedrock: BlockStateId,
        deepslate: BlockStateId,
    ) -> Self {
        let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, seed);
        let shape = GenerationShape::overworld();

        Self {
            proto_routers,
            shape,
            seed,
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

impl ChunkGenerator for PumpkinNoiseGenerator {
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

        let h_count = self.shape.horizontal_cell_block_count;
        let v_count = self.shape.vertical_cell_block_count;
        let horizontal_cells = 16 / h_count;
        let vertical_cell_count = self.shape.height / v_count;
        let minimum_cell_y = floor_div(self.shape.min_y, v_count);

        let delta_y_step = 1.0 / f64::from(v_count);
        let delta_xz_step = 1.0 / f64::from(h_count);

        // Build chunk-specific noise router
        let horizontal_biome_end = ((horizontal_cells + 1) * h_count) as usize >> 2;
        let builder_options = ChunkNoiseFunctionBuilderOptions::new(
            h_count as usize,
            v_count as usize,
            vertical_cell_count as usize,
            (horizontal_cells + 1) as usize,
            base_x >> 2,
            base_z >> 2,
            horizontal_biome_end,
        );

        let mut router = ChunkNoiseRouter::generate(&self.proto_routers.noise, &builder_options);

        let start_cell_x = floor_div(base_x, h_count);
        let start_cell_z = floor_div(base_z, h_count);

        // Track cache IDs like Pumpkin does
        let mut cache_fill_unique_id: u64 = 0;
        let mut cache_result_unique_id: u64 = 0;

        let sample_params = DensitySampleParams {
            current_x: start_cell_x,
            start_cell_z,
            minimum_cell_y,
            h_count,
            v_count,
        };

        // Sample start density column
        Self::sample_density(
            &mut router,
            true,
            &sample_params,
            &mut cache_fill_unique_id,
            &mut cache_result_unique_id,
        );

        for cell_x in 0..horizontal_cells {
            // Sample end density column
            let end_params = DensitySampleParams {
                current_x: start_cell_x + cell_x + 1,
                ..sample_params
            };
            Self::sample_density(
                &mut router,
                false,
                &end_params,
                &mut cache_fill_unique_id,
                &mut cache_result_unique_id,
            );

            let sample_start_x = (start_cell_x + cell_x) * h_count;
            let block_x_base = base_x + cell_x * h_count;

            for cell_z in 0..horizontal_cells {
                let sample_start_z = (start_cell_z + cell_z) * h_count;
                let block_z_base = base_z + cell_z * h_count;

                for cell_y in (0..vertical_cell_count).rev() {
                    // Notify router about cell corners and fill cell caches
                    router.on_sampled_cell_corners(cell_y as usize, cell_z as usize);
                    cache_fill_unique_id += 1;

                    let sample_start_y = (minimum_cell_y + cell_y) * v_count;

                    // Fill cell caches with the ChunkIndexMapper
                    let mapper = ChunkIndexMapper {
                        start_x: sample_start_x,
                        start_y: sample_start_y,
                        start_z: sample_start_z,
                        horizontal_cell_block_count: h_count as usize,
                        vertical_cell_block_count: v_count as usize,
                    };

                    let mut cell_cache_options = ChunkNoiseFunctionSampleOptions::new(
                        true, // populating_caches = true
                        SampleAction::CellCaches(WrapperData::new(
                            0,
                            0,
                            0,
                            h_count as usize,
                            v_count as usize,
                        )),
                        cache_result_unique_id,
                        cache_fill_unique_id,
                        0,
                    );

                    router.fill_cell_caches(&mapper, &mut cell_cache_options);
                    cache_fill_unique_id += 1;

                    for local_y in (0..v_count).rev() {
                        let block_y = sample_start_y + local_y;
                        router.interpolate_y(f64::from(local_y) * delta_y_step);

                        for local_x in 0..h_count {
                            router.interpolate_x(f64::from(local_x) * delta_xz_step);
                            let block_x = block_x_base + local_x;

                            for local_z in 0..h_count {
                                cache_result_unique_id += 1;
                                router.interpolate_z(f64::from(local_z) * delta_xz_step);
                                let block_z = block_z_base + local_z;

                                let cell_offset_x = local_x;
                                let cell_offset_y = block_y - sample_start_y;
                                let cell_offset_z = local_z;

                                // Sample final density using interpolated value
                                let pos = UnblendedNoisePos::new(
                                    sample_start_x + cell_offset_x,
                                    sample_start_y + cell_offset_y,
                                    sample_start_z + cell_offset_z,
                                );
                                let sample_options = ChunkNoiseFunctionSampleOptions::new(
                                    false,
                                    SampleAction::CellCaches(WrapperData::new(
                                        cell_offset_x as usize,
                                        cell_offset_y as usize,
                                        cell_offset_z as usize,
                                        h_count as usize,
                                        v_count as usize,
                                    )),
                                    cache_result_unique_id,
                                    cache_fill_unique_id,
                                    0,
                                );

                                let density = router.final_density(&pos, &sample_options);

                                // Determine block based on density
                                let world_y = block_y;
                                let local_y_idx = (block_y - self.shape.min_y) as usize;
                                let local_x_idx = (block_x - base_x) as usize;
                                let local_z_idx = (block_z - base_z) as usize;

                                let bedrock_top = self.shape.min_y + 5;
                                let is_solid = density > 0.0;

                                let block = if world_y == self.shape.min_y {
                                    self.bedrock
                                } else if world_y < bedrock_top {
                                    let hash = position_hash(block_x, world_y, block_z);
                                    let bedrock_chance = (bedrock_top - world_y) as u32;
                                    if hash % 5 < bedrock_chance {
                                        self.bedrock
                                    } else if is_solid {
                                        if world_y < 0 {
                                            self.deepslate
                                        } else {
                                            self.stone
                                        }
                                    } else if world_y <= SEA_LEVEL {
                                        self.water
                                    } else {
                                        continue;
                                    }
                                } else if is_solid {
                                    if world_y < 0 {
                                        self.deepslate
                                    } else {
                                        self.stone
                                    }
                                } else if world_y <= SEA_LEVEL {
                                    self.water
                                } else {
                                    continue;
                                };

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
            router.swap_buffers();
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

/// Parameters for sampling density in a cell column.
struct DensitySampleParams {
    current_x: i32,
    start_cell_z: i32,
    minimum_cell_y: i32,
    h_count: i32,
    v_count: i32,
}

impl PumpkinNoiseGenerator {
    /// Sample density for a column of cells at the given x position.
    fn sample_density(
        router: &mut ChunkNoiseRouter,
        start: bool,
        params: &DensitySampleParams,
        cache_fill_unique_id: &mut u64,
        cache_result_unique_id: &mut u64,
    ) {
        let x = params.current_x * params.h_count;

        for cell_z in 0..=(16 / params.h_count) {
            let current_cell_z = params.start_cell_z + cell_z;
            let z = current_cell_z * params.h_count;
            *cache_fill_unique_id += 1;

            let mapper = InterpolationIndexMapper {
                x,
                z,
                minimum_cell_y: params.minimum_cell_y,
                vertical_cell_block_count: params.v_count,
            };

            let mut options = ChunkNoiseFunctionSampleOptions::new(
                false,
                SampleAction::CellCaches(WrapperData::new(
                    0,
                    0,
                    0,
                    params.h_count as usize,
                    params.v_count as usize,
                )),
                *cache_result_unique_id,
                *cache_fill_unique_id,
                0,
            );

            router.fill_interpolator_buffers(start, cell_z as usize, &mapper, &mut options);
            *cache_result_unique_id = options.cache_result_unique_id;
        }
        *cache_fill_unique_id += 1;
    }
}

/// Maps indices to noise positions for interpolation buffer filling.
struct InterpolationIndexMapper {
    x: i32,
    z: i32,
    minimum_cell_y: i32,
    vertical_cell_block_count: i32,
}

impl IndexToNoisePos for InterpolationIndexMapper {
    fn at(
        &self,
        index: usize,
        sample_data: Option<&mut ChunkNoiseFunctionSampleOptions>,
    ) -> impl NoisePos + 'static {
        if let Some(sample_data) = sample_data {
            sample_data.cache_result_unique_id += 1;
            sample_data.fill_index = index;
        }

        let y = (index as i32 + self.minimum_cell_y) * self.vertical_cell_block_count;
        UnblendedNoisePos::new(self.x, y, self.z)
    }
}

/// Maps cell indices to noise positions for cell cache filling.
struct ChunkIndexMapper {
    start_x: i32,
    start_y: i32,
    start_z: i32,
    horizontal_cell_block_count: usize,
    vertical_cell_block_count: usize,
}

impl IndexToNoisePos for ChunkIndexMapper {
    fn at(
        &self,
        index: usize,
        sample_options: Option<&mut ChunkNoiseFunctionSampleOptions>,
    ) -> impl NoisePos + 'static {
        let cell_z_position = floor_mod(index, self.horizontal_cell_block_count);
        let xy_portion = index / self.horizontal_cell_block_count;
        let cell_x_position = floor_mod(xy_portion, self.horizontal_cell_block_count);
        let cell_y_position =
            self.vertical_cell_block_count - 1 - (xy_portion / self.horizontal_cell_block_count);

        if let Some(sample_options) = sample_options {
            sample_options.fill_index = index;
            if let SampleAction::CellCaches(wrapper_data) = &mut sample_options.action {
                wrapper_data.update_position(cell_x_position, cell_y_position, cell_z_position);
            }
        }

        UnblendedNoisePos::new(
            self.start_x + cell_x_position as i32,
            self.start_y + cell_y_position as i32,
            self.start_z + cell_z_position as i32,
        )
    }
}
