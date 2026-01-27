//! Surface height estimation for terrain generation.
//!
//! This module provides height estimation for the aquifer system to determine
//! where the surface is relative to underground features.

use std::collections::HashMap;

use super::chunk_density_function::{
    biome_coords, Cache2D, ChunkNoiseFunctionSampleOptions, ChunkSpecificNoiseFunctionComponent,
    FlatCache, SampleAction,
};
use super::chunk_noise_router::ChunkNoiseFunctionComponent;
use super::density_function::{NoiseFunctionComponentRange, PassThrough, UnblendedNoisePos};
use super::proto_noise_router::{ProtoNoiseFunctionComponent, ProtoSurfaceEstimator};
use super::WrapperType;

/// Density cutoff for surface detection.
///
/// Vanilla's `FindTopSurface.compute()` checks `density > 0.0`. The density function
/// already has `-0.390625` baked in (from `NoiseRouterData`), so checking `> 0.0` is
/// equivalent to checking the raw density `> 0.390625`.
const SURFACE_DENSITY_CUTOFF: f64 = 0.0;

/// Options for building the surface height sampler.
pub struct SurfaceHeightSamplerBuilderOptions {
    /// Starting biome X coordinate.
    pub start_biome_x: i32,
    /// Starting biome Z coordinate.
    pub start_biome_z: i32,
    /// Number of biome regions per chunk axis.
    pub horizontal_biome_end: usize,
    /// Minimum Y level to search.
    pub minimum_y: i32,
    /// Maximum Y level to search.
    pub maximum_y: i32,
    /// Vertical step count for search.
    pub y_level_step_count: i32,
}

impl SurfaceHeightSamplerBuilderOptions {
    /// Creates new builder options.
    #[must_use]
    pub const fn new(
        start_biome_x: i32,
        start_biome_z: i32,
        horizontal_biome_end: usize,
        minimum_y: i32,
        maximum_y: i32,
        y_level_step_count: i32,
    ) -> Self {
        Self {
            start_biome_x,
            start_biome_z,
            horizontal_biome_end,
            minimum_y,
            maximum_y,
            y_level_step_count,
        }
    }
}

/// Surface height estimator for aquifer calculations.
pub struct SurfaceHeightEstimateSampler<'a> {
    /// Component stack for sampling.
    component_stack: Box<[ChunkNoiseFunctionComponent<'a>]>,
    /// Cached height estimates by packed XZ position.
    cache: HashMap<i64, i32>,
    /// Minimum Y level.
    minimum_y: i32,
    /// Maximum Y level.
    maximum_y: i32,
    /// Vertical step count.
    y_level_step_count: i32,
}

impl<'a> SurfaceHeightEstimateSampler<'a> {
    /// Generates a new surface height estimator from the proto estimator.
    #[must_use]
    pub fn generate(
        base: &'a ProtoSurfaceEstimator,
        options: &SurfaceHeightSamplerBuilderOptions,
    ) -> Self {
        let mut component_stack =
            Vec::<ChunkNoiseFunctionComponent>::with_capacity(base.full_component_stack.len());

        for base_component in base.full_component_stack.iter() {
            let chunk_component = match base_component {
                ProtoNoiseFunctionComponent::Dependent(dependent) => {
                    ChunkNoiseFunctionComponent::Dependent(dependent)
                }
                ProtoNoiseFunctionComponent::Independent(independent) => {
                    ChunkNoiseFunctionComponent::Independent(independent)
                }
                ProtoNoiseFunctionComponent::PassThrough(pass_through) => {
                    ChunkNoiseFunctionComponent::PassThrough(*pass_through)
                }
                ProtoNoiseFunctionComponent::Wrapper(wrapper) => {
                    let min_value = component_stack[wrapper.input_index].min();
                    let max_value = component_stack[wrapper.input_index].max();

                    match wrapper.wrapper_type {
                        WrapperType::Cache2D => ChunkNoiseFunctionComponent::Chunk(
                            ChunkSpecificNoiseFunctionComponent::Cache2D(Cache2D::new(
                                wrapper.input_index,
                                min_value,
                                max_value,
                            )),
                        ),
                        WrapperType::CacheFlat => {
                            let mut flat_cache = FlatCache::new(
                                wrapper.input_index,
                                min_value,
                                max_value,
                                options.start_biome_x,
                                options.start_biome_z,
                                options.horizontal_biome_end,
                            );
                            let sample_options = ChunkNoiseFunctionSampleOptions::new(
                                false,
                                SampleAction::SkipCellCaches,
                                0,
                                0,
                                0,
                            );

                            // Pre-fill the flat cache
                            for biome_x in 0..=options.horizontal_biome_end {
                                let abs_biome_x = options.start_biome_x + biome_x as i32;
                                let block_x = biome_coords::to_block(abs_biome_x);

                                for biome_z in 0..=options.horizontal_biome_end {
                                    let abs_biome_z = options.start_biome_z + biome_z as i32;
                                    let block_z = biome_coords::to_block(abs_biome_z);

                                    let pos = UnblendedNoisePos::new(block_x, 0, block_z);
                                    let sample = ChunkNoiseFunctionComponent::sample_from_stack(
                                        &mut component_stack[..=wrapper.input_index],
                                        &pos,
                                        &sample_options,
                                    );

                                    let cache_index =
                                        flat_cache.xz_to_index_const(biome_x, biome_z);
                                    flat_cache.cache[cache_index] = sample;
                                }
                            }

                            ChunkNoiseFunctionComponent::Chunk(
                                ChunkSpecificNoiseFunctionComponent::FlatCache(flat_cache),
                            )
                        }
                        // Surface estimator doesn't use interpolation or cell caches
                        _ => ChunkNoiseFunctionComponent::PassThrough(PassThrough::new(
                            wrapper.input_index,
                            min_value,
                            max_value,
                        )),
                    }
                }
            };
            component_stack.push(chunk_component);
        }

        Self {
            component_stack: component_stack.into_boxed_slice(),
            cache: HashMap::new(),
            minimum_y: options.minimum_y,
            maximum_y: options.maximum_y,
            y_level_step_count: options.y_level_step_count,
        }
    }

    /// Estimates the surface height at the given block coordinates.
    ///
    /// Coordinates are quantized to 4-block boundaries (quart pos) for caching,
    /// matching vanilla's behavior.
    pub fn estimate_height(&mut self, x: i32, z: i32) -> i32 {
        // Quantize to 4-block (quart pos) boundaries like vanilla
        let quantized_x = (x >> 2) << 2;
        let quantized_z = (z >> 2) << 2;

        // Pack coordinates for cache key
        let packed = pack_xz(quantized_x, quantized_z);

        if let Some(&height) = self.cache.get(&packed) {
            return height;
        }

        let height = self.compute_height(quantized_x, quantized_z);
        self.cache.insert(packed, height);
        height
    }

    /// Computes the surface height using top-down linear search.
    ///
    /// This matches vanilla's `FindTopSurface` algorithm: search from top down,
    /// stepping by cellHeight (y_level_step_count), returning the highest Y
    /// where density > threshold.
    fn compute_height(&mut self, x: i32, z: i32) -> i32 {
        let sample_options = ChunkNoiseFunctionSampleOptions::new(
            false,
            SampleAction::SkipCellCaches,
            0,
            0,
            0,
        );

        // Align maximum_y to cell boundary (step size)
        let top_y = (self.maximum_y / self.y_level_step_count) * self.y_level_step_count;

        if top_y <= self.minimum_y {
            return self.minimum_y;
        }

        // Linear search from top down, stepping by cell height
        // Returns the HIGHEST Y where density > threshold (solid terrain)
        let mut block_y = top_y;
        while block_y >= self.minimum_y {
            let pos = UnblendedNoisePos::new(x, block_y, z);
            let density = ChunkNoiseFunctionComponent::sample_from_stack(
                &mut self.component_stack,
                &pos,
                &sample_options,
            );

            if density > SURFACE_DENSITY_CUTOFF {
                return block_y;
            }

            block_y -= self.y_level_step_count;
        }

        self.minimum_y
    }
}

/// Packs X and Z coordinates into a single i64 for caching.
#[inline]
fn pack_xz(x: i32, z: i32) -> i64 {
    (i64::from(x) & 0xFFFF_FFFF) | ((i64::from(z) & 0xFFFF_FFFF) << 32)
}
