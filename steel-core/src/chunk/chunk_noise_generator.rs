//! Chunk noise generator for terrain generation.
//!
//! This module provides a high-level wrapper around the noise router and
//! block state samplers for chunk terrain generation.

// Uses coordinate variables (cell_x, cell_y, cell_z, etc.)
#![allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]

use steel_utils::BlockStateId;
use steel_utils::noise::floor_div;
use steel_utils::noise_router::{
    AquiferBlocks, AquiferSampler, ChainedBlockStateSampler, ChunkNoiseFunctionBuilderOptions,
    ChunkNoiseFunctionSampleOptions, ChunkNoiseRouter, FluidLevelSampler, IndexToNoisePos,
    NoisePosTraitAlias as NoisePosTrait, OreBlocks, OreVeinSampler, ProtoNoiseRouter,
    ProtoSurfaceEstimator, SampleAction, SeaLevelAquiferSampler, SurfaceHeightEstimateSampler,
    SurfaceHeightSamplerBuilderOptions, UnblendedNoisePos, WorldAquiferSampler, WrapperData,
};

use super::random_config::WorldRandomConfig;

/// Converts block coordinates to biome coordinates.
#[inline]
fn biome_from_block(block: i32) -> i32 {
    block >> 2
}

/// Converts block coordinates to section coordinates.
#[inline]
fn block_to_section(block: i32) -> i32 {
    block >> 4
}

/// Generation shape configuration for a dimension.
///
/// Defines the vertical bounds and cell sizes for terrain generation.
/// Different dimensions use different configurations:
///
/// | Dimension | Min Y | Height | Horizontal Cell | Vertical Cell |
/// |-----------|-------|--------|-----------------|---------------|
/// | Overworld | -64   | 384    | 4               | 8             |
/// | Nether    | 0     | 128    | 4               | 8             |
/// | The End   | 0     | 256    | 4               | 4             |
#[derive(Clone)]
pub struct GenerationShapeConfig {
    /// Minimum Y coordinate (e.g., -64 for overworld).
    pub min_y: i8,
    /// Total height in blocks (e.g., 384 for overworld).
    pub height: u16,
    /// Horizontal cell size in blocks (typically 4).
    horizontal_cell_block_count: u8,
    /// Vertical cell size in blocks (typically 8).
    vertical_cell_block_count: u8,
}

impl GenerationShapeConfig {
    /// Creates a new generation shape configuration.
    #[must_use]
    pub const fn new(
        min_y: i8,
        height: u16,
        horizontal_cell_block_count: u8,
        vertical_cell_block_count: u8,
    ) -> Self {
        Self {
            min_y,
            height,
            horizontal_cell_block_count,
            vertical_cell_block_count,
        }
    }

    /// Returns the overworld generation shape.
    #[must_use]
    pub const fn overworld() -> Self {
        Self {
            min_y: -64,
            height: 384,
            horizontal_cell_block_count: 4,
            vertical_cell_block_count: 8,
        }
    }

    /// Returns the horizontal cell block count.
    #[must_use]
    pub const fn horizontal_cell_block_count(&self) -> u8 {
        self.horizontal_cell_block_count
    }

    /// Returns the vertical cell block count.
    #[must_use]
    pub const fn vertical_cell_block_count(&self) -> u8 {
        self.vertical_cell_block_count
    }
}

/// Block state IDs needed for terrain generation.
///
/// Contains all block types used during the noise-based terrain
/// generation phase, including stone, fluids, and ore vein blocks.
#[derive(Clone)]
pub struct TerrainBlocks {
    /// Default solid block.
    pub stone: BlockStateId,
    /// Water block for seas and aquifers.
    pub water: BlockStateId,
    /// Lava block for deep underground and aquifers.
    pub lava: BlockStateId,
    /// Air block for caves and surface.
    pub air: BlockStateId,
    /// Bedrock block.
    pub bedrock: BlockStateId,
    /// Copper ore (Y >= 0).
    pub copper_ore: BlockStateId,
    /// Deepslate copper ore (Y < 0).
    pub deepslate_copper_ore: BlockStateId,
    /// Raw copper block (rare in veins).
    pub raw_copper_block: BlockStateId,
    /// Granite (copper vein filler).
    pub granite: BlockStateId,
    /// Iron ore (Y >= 0).
    pub iron_ore: BlockStateId,
    /// Deepslate iron ore (Y < 0).
    pub deepslate_iron_ore: BlockStateId,
    /// Raw iron block (rare in veins).
    pub raw_iron_block: BlockStateId,
    /// Tuff (iron vein filler).
    pub tuff: BlockStateId,
}

impl TerrainBlocks {
    /// Converts to aquifer blocks.
    #[must_use]
    pub fn to_aquifer_blocks(&self) -> AquiferBlocks {
        AquiferBlocks {
            water: self.water,
            lava: self.lava,
            air: self.air,
        }
    }

    /// Converts to ore blocks.
    #[must_use]
    pub fn to_ore_blocks(&self) -> OreBlocks {
        OreBlocks {
            copper_ore: self.copper_ore,
            deepslate_copper_ore: self.deepslate_copper_ore,
            raw_copper_block: self.raw_copper_block,
            granite: self.granite,
            iron_ore: self.iron_ore,
            deepslate_iron_ore: self.deepslate_iron_ore,
            raw_iron_block: self.raw_iron_block,
            tuff: self.tuff,
        }
    }
}

/// Chunk noise generator managing the router and samplers.
///
/// This is the high-level coordinator for terrain generation within a chunk.
/// It combines:
/// - The noise router for density evaluation
/// - The block state sampler chain (aquifer + ore veins)
/// - Surface height estimation for aquifer calculations
///
/// # Usage
///
/// 1. Create with `new()`
/// 2. Call `sample_start_density()` once
/// 3. For each X column:
///    - Call `sample_end_density(cell_x)`
///    - For each Z, Y cell: call `on_sampled_cell_corners()`
///    - For each block: call `interpolate_y/x/z()` then `sample_block_state()`
///    - Call `swap_buffers()` at end of column
pub struct ChunkNoiseGenerator<'a> {
    /// Block state sampler chain (ore veins -> aquifer -> default).
    pub state_sampler: ChainedBlockStateSampler,
    /// Surface height estimator for aquifer fluid level calculation.
    pub height_estimator: SurfaceHeightEstimateSampler<'a>,
    /// The chunk noise router for density evaluation.
    pub router: ChunkNoiseRouter<'a>,
    /// Generation shape configuration.
    generation_shape: &'a GenerationShapeConfig,
    /// Block state IDs for terrain.
    blocks: &'a TerrainBlocks,
    /// Starting cell X position.
    start_cell_pos_x: i32,
    /// Starting cell Z position.
    start_cell_pos_z: i32,
    /// Number of vertical cells.
    vertical_cell_count: usize,
    /// Minimum cell Y coordinate.
    minimum_cell_y: i32,
    /// Cache fill unique ID (incremented per fill operation).
    cache_fill_unique_id: u64,
    /// Cache result unique ID (incremented per sample).
    cache_result_unique_id: u64,
}

impl<'a> ChunkNoiseGenerator<'a> {
    /// Creates a new chunk noise generator for terrain generation.
    ///
    /// This is the main entry point for creating a generator that will fill
    /// a chunk with terrain using noise-based density functions.
    ///
    /// # Arguments
    ///
    /// * `noise_router_base` - The proto noise router (seed-initialized)
    /// * `surface_estimator_base` - Surface height estimation router for aquifers
    /// * `random_config` - World random configuration for aquifer/ore derivers
    /// * `horizontal_cell_count` - Number of cells horizontally (typically 4)
    /// * `start_block_x` - Starting X block coordinate of the chunk
    /// * `start_block_z` - Starting Z block coordinate of the chunk
    /// * `generation_shape` - Generation bounds (`min_y`, height, cell sizes)
    /// * `fluid_level_sampler` - Default fluid levels (sea level, lava level)
    /// * `blocks` - Block state IDs for terrain generation
    /// * `enable_aquifers` - Whether to enable underground water/lava pockets
    /// * `enable_ore_veins` - Whether to enable large ore vein generation
    ///
    /// # Returns
    ///
    /// A new `ChunkNoiseGenerator` ready for terrain generation.
    #[must_use] 
    pub fn new(
        noise_router_base: &'a ProtoNoiseRouter,
        surface_estimator_base: &'a ProtoSurfaceEstimator,
        random_config: &WorldRandomConfig,
        horizontal_cell_count: usize,
        start_block_x: i32,
        start_block_z: i32,
        generation_shape: &'a GenerationShapeConfig,
        fluid_level_sampler: FluidLevelSampler,
        blocks: &'a TerrainBlocks,
        enable_aquifers: bool,
        enable_ore_veins: bool,
    ) -> Self {
        let h_cell = generation_shape.horizontal_cell_block_count();
        let v_cell = generation_shape.vertical_cell_block_count();

        let start_cell_pos_x = floor_div(start_block_x, i32::from(h_cell));
        let start_cell_pos_z = floor_div(start_block_z, i32::from(h_cell));

        let horizontal_biome_end =
            biome_from_block((horizontal_cell_count * h_cell as usize) as i32) as usize;
        let vertical_cell_count =
            floor_div(i32::from(generation_shape.height), i32::from(v_cell)) as usize;
        let minimum_cell_y = floor_div(i32::from(generation_shape.min_y), i32::from(v_cell));

        // Build chunk noise router
        let builder_options = ChunkNoiseFunctionBuilderOptions::new(
            h_cell as usize,
            v_cell as usize,
            vertical_cell_count,
            horizontal_cell_count,
            biome_from_block(start_block_x),
            biome_from_block(start_block_z),
            horizontal_biome_end,
        );

        let router = ChunkNoiseRouter::generate(noise_router_base, &builder_options);

        // Build surface height estimator
        let height_options = SurfaceHeightSamplerBuilderOptions::new(
            biome_from_block(start_block_x),
            biome_from_block(start_block_z),
            horizontal_biome_end,
            i32::from(generation_shape.min_y),
            i32::from(generation_shape.min_y) + i32::from(generation_shape.height),
            i32::from(v_cell),
        );
        let height_estimator =
            SurfaceHeightEstimateSampler::generate(surface_estimator_base, &height_options);

        // Build aquifer sampler
        let aquifer_sampler = if enable_aquifers {
            let section_x = block_to_section(start_block_x);
            let section_z = block_to_section(start_block_z);
            AquiferSampler::World(WorldAquiferSampler::new(
                section_x,
                section_z,
                random_config.aquifer_deriver,
                generation_shape.min_y,
                generation_shape.height,
                fluid_level_sampler,
                blocks.to_aquifer_blocks(),
            ))
        } else {
            AquiferSampler::SeaLevel(SeaLevelAquiferSampler::new(
                fluid_level_sampler,
                blocks.to_aquifer_blocks(),
            ))
        };

        // Build chained sampler
        let state_sampler = if enable_ore_veins {
            let ore_sampler =
                OreVeinSampler::new(random_config.ore_deriver, blocks.to_ore_blocks());
            ChainedBlockStateSampler::with_ores(aquifer_sampler, ore_sampler)
        } else {
            ChainedBlockStateSampler::aquifer_only(aquifer_sampler)
        };

        Self {
            state_sampler,
            height_estimator,
            router,
            generation_shape,
            blocks,
            start_cell_pos_x,
            start_cell_pos_z,
            vertical_cell_count,
            minimum_cell_y,
            cache_fill_unique_id: 0,
            cache_result_unique_id: 0,
        }
    }

    /// Samples the start density column for trilinear interpolation.
    ///
    /// Call this once before starting the generation loop. This fills the
    /// start buffer with density values at the chunk's first X column.
    #[inline]
    pub fn sample_start_density(&mut self) {
        self.cache_result_unique_id = 0;
        self.sample_density(true, self.start_cell_pos_x);
    }

    /// Samples the end density column for the given cell X index.
    ///
    /// Call this at the start of each X iteration to sample the next column.
    /// After processing, call `swap_buffers()` to reuse this as the start column.
    ///
    /// # Arguments
    ///
    /// * `cell_x` - The current cell X index (0 to horizontal_cells-1)
    #[inline]
    pub fn sample_end_density(&mut self, cell_x: i32) {
        self.sample_density(false, self.start_cell_pos_x + cell_x + 1);
    }

    /// Internal: Samples density for a vertical column at the given X position.
    ///
    /// This fills interpolator buffers with density values for all Y levels
    /// at each Z cell position. The `start` flag determines whether to fill
    /// the start buffer or end buffer.
    fn sample_density(&mut self, start: bool, current_x: i32) {
        let h_cell = i32::from(self.generation_shape.horizontal_cell_block_count());
        let v_cell = i32::from(self.generation_shape.vertical_cell_block_count());
        let x = current_x * h_cell;

        for cell_z in 0..=(16 / h_cell) {
            let current_cell_z_pos = self.start_cell_pos_z + cell_z;
            let z = current_cell_z_pos * h_cell;
            self.cache_fill_unique_id += 1;

            let mapper = InterpolationIndexMapper {
                x,
                z,
                minimum_cell_y: self.minimum_cell_y,
                vertical_cell_block_count: v_cell,
            };

            let mut options = ChunkNoiseFunctionSampleOptions::new(
                false,
                SampleAction::CellCaches(WrapperData::new(
                    0,
                    0,
                    0,
                    h_cell as usize,
                    v_cell as usize,
                )),
                self.cache_result_unique_id,
                self.cache_fill_unique_id,
                0,
            );

            self.router
                .fill_interpolator_buffers(start, cell_z as usize, &mapper, &mut options);
            self.cache_result_unique_id = options.cache_result_unique_id;
        }
        self.cache_fill_unique_id += 1;
    }

    /// Interpolates in the X direction.
    #[inline]
    pub fn interpolate_x(&mut self, delta: f64) {
        self.router.interpolate_x(delta);
    }

    /// Interpolates in the Y direction.
    #[inline]
    pub fn interpolate_y(&mut self, delta: f64) {
        self.router.interpolate_y(delta);
    }

    /// Interpolates in the Z direction.
    #[inline]
    pub fn interpolate_z(&mut self, delta: f64) {
        self.cache_result_unique_id += 1;
        self.router.interpolate_z(delta);
    }

    /// Swaps the interpolator buffers.
    #[inline]
    pub fn swap_buffers(&mut self) {
        self.router.swap_buffers();
    }

    /// Notifies the generator that cell corners have been sampled.
    ///
    /// This must be called at the start of each cell iteration to:
    /// 1. Extract the 8 corner values from interpolator buffers
    /// 2. Fill cell caches with density values for this cell
    ///
    /// # Arguments
    ///
    /// * `cell_x` - Cell X index (0 to horizontal_cells-1)
    /// * `cell_y` - Cell Y index (0 to vertical_cell_count-1)
    /// * `cell_z` - Cell Z index (0 to horizontal_cells-1)
    pub fn on_sampled_cell_corners(&mut self, cell_x: i32, cell_y: i32, cell_z: i32) {
        let h_cell = self.generation_shape.horizontal_cell_block_count() as usize;
        let v_cell = self.generation_shape.vertical_cell_block_count() as usize;

        self.router
            .on_sampled_cell_corners(cell_y as usize, cell_z as usize);
        self.cache_fill_unique_id += 1;

        let start_x = (self.start_cell_pos_x + cell_x)
            * i32::from(self.generation_shape.horizontal_cell_block_count());
        let start_y = (cell_y + self.minimum_cell_y)
            * i32::from(self.generation_shape.vertical_cell_block_count());
        let start_z = (self.start_cell_pos_z + cell_z)
            * i32::from(self.generation_shape.horizontal_cell_block_count());

        let mapper = ChunkIndexMapper {
            start_x,
            start_y,
            start_z,
            horizontal_cell_block_count: h_cell,
            vertical_cell_block_count: v_cell,
        };

        let mut sample_options = ChunkNoiseFunctionSampleOptions::new(
            true,
            SampleAction::CellCaches(WrapperData::new(0, 0, 0, h_cell, v_cell)),
            self.cache_result_unique_id,
            self.cache_fill_unique_id,
            0,
        );

        self.router.fill_cell_caches(&mapper, &mut sample_options);
        self.cache_fill_unique_id += 1;
    }

    /// Samples the block state at a position.
    pub fn sample_block_state(
        &mut self,
        start_x: i32,
        start_y: i32,
        start_z: i32,
        cell_x: i32,
        cell_y: i32,
        cell_z: i32,
    ) -> Option<BlockStateId> {
        let h_cell = self.generation_shape.horizontal_cell_block_count() as usize;
        let v_cell = self.generation_shape.vertical_cell_block_count() as usize;

        let pos = UnblendedNoisePos::new(start_x + cell_x, start_y + cell_y, start_z + cell_z);

        let options = ChunkNoiseFunctionSampleOptions::new(
            false,
            SampleAction::CellCaches(WrapperData::new(
                cell_x as usize,
                cell_y as usize,
                cell_z as usize,
                h_cell,
                v_cell,
            )),
            self.cache_result_unique_id,
            self.cache_fill_unique_id,
            0,
        );

        self.state_sampler
            .sample(&mut self.router, &pos, &options, &mut self.height_estimator)
    }

    /// Returns the horizontal cell block count.
    #[inline]
    #[must_use] 
    pub fn horizontal_cell_block_count(&self) -> u8 {
        self.generation_shape.horizontal_cell_block_count()
    }

    /// Returns the vertical cell block count.
    #[inline]
    #[must_use] 
    pub fn vertical_cell_block_count(&self) -> u8 {
        self.generation_shape.vertical_cell_block_count()
    }

    /// Returns the minimum Y.
    #[inline]
    #[must_use] 
    pub fn min_y(&self) -> i8 {
        self.generation_shape.min_y
    }

    /// Returns the height.
    #[inline]
    #[must_use] 
    pub fn height(&self) -> u16 {
        self.generation_shape.height
    }

    /// Returns the blocks configuration.
    #[inline]
    #[must_use] 
    pub fn blocks(&self) -> &TerrainBlocks {
        self.blocks
    }

    /// Returns the vertical cell count.
    #[inline]
    #[must_use] 
    pub fn vertical_cell_count(&self) -> usize {
        self.vertical_cell_count
    }

    /// Returns the minimum cell Y.
    #[inline]
    #[must_use] 
    pub fn minimum_cell_y(&self) -> i32 {
        self.minimum_cell_y
    }

    /// Returns the start cell X position.
    #[inline]
    #[must_use] 
    pub fn start_cell_pos_x(&self) -> i32 {
        self.start_cell_pos_x
    }

    /// Returns the start cell Z position.
    #[inline]
    #[must_use] 
    pub fn start_cell_pos_z(&self) -> i32 {
        self.start_cell_pos_z
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
    ) -> impl NoisePosTrait + 'static {
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

/// Floor modulo for usize values.
#[inline]
fn floor_mod_usize(a: usize, b: usize) -> usize {
    ((a % b) + b) % b
}

impl IndexToNoisePos for ChunkIndexMapper {
    fn at(
        &self,
        index: usize,
        sample_options: Option<&mut ChunkNoiseFunctionSampleOptions>,
    ) -> impl NoisePosTrait + 'static {
        let cell_z_position = floor_mod_usize(index, self.horizontal_cell_block_count);
        let xy_portion = index / self.horizontal_cell_block_count;
        let cell_x_position = floor_mod_usize(xy_portion, self.horizontal_cell_block_count);
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
