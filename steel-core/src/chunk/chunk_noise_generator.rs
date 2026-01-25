//! Chunk noise generator for terrain generation.
//!
//! This module provides a high-level wrapper around the noise router and
//! block state samplers for chunk terrain generation.

// Uses coordinate variables (cell_x, cell_y, cell_z, etc.)
#![allow(clippy::similar_names, clippy::too_many_lines, clippy::too_many_arguments)]

use steel_utils::noise::floor_div;
use steel_utils::noise_router::{
    AquiferBlocks, AquiferSampler, ChainedBlockStateSampler, ChunkNoiseFunctionBuilderOptions,
    ChunkNoiseFunctionSampleOptions, ChunkNoiseRouter, FluidLevelSampler, IndexToNoisePos,
    NoisePosTraitAlias as NoisePosTrait, OreBlocks, OreVeinSampler, ProtoNoiseRouter,
    ProtoSurfaceEstimator, SampleAction, SeaLevelAquiferSampler, SurfaceHeightEstimateSampler,
    SurfaceHeightSamplerBuilderOptions, UnblendedNoisePos, WorldAquiferSampler, WrapperData,
};
use steel_utils::BlockStateId;

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

/// Generation shape configuration.
#[derive(Clone)]
pub struct GenerationShapeConfig {
    /// Minimum Y coordinate.
    pub min_y: i8,
    /// Total height.
    pub height: u16,
    /// Horizontal cell block count (typically 4).
    horizontal_cell_block_count: u8,
    /// Vertical cell block count (typically 8).
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
#[derive(Clone)]
pub struct TerrainBlocks {
    /// Stone block.
    pub stone: BlockStateId,
    /// Deepslate block.
    pub deepslate: BlockStateId,
    /// Water block.
    pub water: BlockStateId,
    /// Lava block.
    pub lava: BlockStateId,
    /// Air block.
    pub air: BlockStateId,
    /// Bedrock block.
    pub bedrock: BlockStateId,
    /// Copper ore block.
    pub copper_ore: BlockStateId,
    /// Deepslate copper ore block.
    pub deepslate_copper_ore: BlockStateId,
    /// Raw copper block.
    pub raw_copper_block: BlockStateId,
    /// Granite block.
    pub granite: BlockStateId,
    /// Iron ore block.
    pub iron_ore: BlockStateId,
    /// Deepslate iron ore block.
    pub deepslate_iron_ore: BlockStateId,
    /// Raw iron block.
    pub raw_iron_block: BlockStateId,
    /// Tuff block.
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
pub struct ChunkNoiseGenerator<'a> {
    /// Block state sampler chain.
    pub state_sampler: ChainedBlockStateSampler,
    /// Surface height estimator.
    pub height_estimator: SurfaceHeightEstimateSampler<'a>,
    /// The noise router.
    pub router: ChunkNoiseRouter<'a>,
    /// Generation shape.
    generation_shape: &'a GenerationShapeConfig,
    /// Block state IDs.
    blocks: &'a TerrainBlocks,
    /// Starting cell X position.
    start_cell_pos_x: i32,
    /// Starting cell Z position.
    start_cell_pos_z: i32,
    /// Vertical cell count.
    vertical_cell_count: usize,
    /// Minimum cell Y.
    minimum_cell_y: i32,
    /// Cache fill unique ID.
    cache_fill_unique_id: u64,
    /// Cache result unique ID.
    cache_result_unique_id: u64,
}

impl<'a> ChunkNoiseGenerator<'a> {
    /// Creates a new chunk noise generator.
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
        let vertical_cell_count = floor_div(
            i32::from(generation_shape.height) as i32,
            i32::from(v_cell),
        ) as usize;
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
        let mut height_estimator =
            SurfaceHeightEstimateSampler::generate(surface_estimator_base, &height_options);

        // Compute max surface Y for aquifer skip sampling optimization
        // Sample in a grid with 4-block steps (quart pos) like vanilla
        let min_x = start_block_x;
        let min_z = start_block_z;
        let max_x = start_block_x + (horizontal_cell_count as i32 - 1) * h_cell as i32 + 9;
        let max_z = start_block_z + (horizontal_cell_count as i32 - 1) * h_cell as i32 + 9;

        let mut max_surface_y = i32::MIN;
        let mut block_z = min_z;
        while block_z <= max_z {
            let mut block_x = min_x;
            while block_x <= max_x {
                let surface_y = height_estimator.estimate_height(block_x, block_z);
                if surface_y > max_surface_y {
                    max_surface_y = surface_y;
                }
                block_x += 4;
            }
            block_z += 4;
        }

        // Build aquifer sampler
        let aquifer_sampler = if enable_aquifers {
            let section_x = block_to_section(start_block_x);
            let section_z = block_to_section(start_block_z);
            AquiferSampler::World(WorldAquiferSampler::new(
                section_x,
                section_z,
                random_config.aquifer_deriver.clone(),
                generation_shape.min_y,
                generation_shape.height,
                fluid_level_sampler,
                blocks.to_aquifer_blocks(),
                max_surface_y,
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
                OreVeinSampler::new(random_config.ore_deriver.clone(), blocks.to_ore_blocks());
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

    /// Samples the start density column.
    #[inline]
    pub fn sample_start_density(&mut self) {
        self.cache_result_unique_id = 0;
        self.sample_density(true, self.start_cell_pos_x);
    }

    /// Samples the end density column for the given cell X.
    #[inline]
    pub fn sample_end_density(&mut self, cell_x: i32) {
        self.sample_density(false, self.start_cell_pos_x + cell_x + 1);
    }

    /// Samples density for a column.
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
                SampleAction::CellCaches(WrapperData::new(0, 0, 0, h_cell as usize, v_cell as usize)),
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

    /// Called when cell corners are sampled.
    pub fn on_sampled_cell_corners(&mut self, cell_x: i32, cell_y: i32, cell_z: i32) {
        let h_cell = self.generation_shape.horizontal_cell_block_count() as usize;
        let v_cell = self.generation_shape.vertical_cell_block_count() as usize;

        self.router
            .on_sampled_cell_corners(cell_y as usize, cell_z as usize);
        self.cache_fill_unique_id += 1;

        let start_x =
            (self.start_cell_pos_x + cell_x) * self.generation_shape.horizontal_cell_block_count() as i32;
        let start_y =
            (cell_y + self.minimum_cell_y) * self.generation_shape.vertical_cell_block_count() as i32;
        let start_z =
            (self.start_cell_pos_z + cell_z) * self.generation_shape.horizontal_cell_block_count() as i32;

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
    pub fn horizontal_cell_block_count(&self) -> u8 {
        self.generation_shape.horizontal_cell_block_count()
    }

    /// Returns the vertical cell block count.
    #[inline]
    pub fn vertical_cell_block_count(&self) -> u8 {
        self.generation_shape.vertical_cell_block_count()
    }

    /// Returns the minimum Y.
    #[inline]
    pub fn min_y(&self) -> i8 {
        self.generation_shape.min_y
    }

    /// Returns the height.
    #[inline]
    pub fn height(&self) -> u16 {
        self.generation_shape.height
    }

    /// Returns the blocks configuration.
    #[inline]
    pub fn blocks(&self) -> &TerrainBlocks {
        self.blocks
    }

    /// Returns the vertical cell count.
    #[inline]
    pub fn vertical_cell_count(&self) -> usize {
        self.vertical_cell_count
    }

    /// Returns the minimum cell Y.
    #[inline]
    pub fn minimum_cell_y(&self) -> i32 {
        self.minimum_cell_y
    }

    /// Returns the start cell X position.
    #[inline]
    pub fn start_cell_pos_x(&self) -> i32 {
        self.start_cell_pos_x
    }

    /// Returns the start cell Z position.
    #[inline]
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
