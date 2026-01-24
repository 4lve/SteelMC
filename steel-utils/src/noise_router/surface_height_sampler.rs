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

/// Density cutoff for surface detection (matches vanilla).
const SURFACE_DENSITY_CUTOFF: f64 = 0.390625;

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
    /// Uses binary search to find the Y level where density crosses the surface threshold.
    pub fn estimate_height(&mut self, x: i32, z: i32) -> i32 {
        // Pack coordinates for cache key
        let packed = pack_xz(x, z);

        if let Some(&height) = self.cache.get(&packed) {
            return height;
        }

        let height = self.compute_height(x, z);
        self.cache.insert(packed, height);
        height
    }

    /// Computes the surface height using binary search.
    fn compute_height(&mut self, x: i32, z: i32) -> i32 {
        let sample_options = ChunkNoiseFunctionSampleOptions::new(
            false,
            SampleAction::SkipCellCaches,
            0,
            0,
            0,
        );

        let mut low = self.minimum_y;
        let mut high = self.maximum_y;

        // Binary search for the surface
        while low < high {
            let mid = low + (high - low) / 2;
            let y = mid + (mid % self.y_level_step_count);

            let pos = UnblendedNoisePos::new(x, y, z);
            let density = ChunkNoiseFunctionComponent::sample_from_stack(
                &mut self.component_stack,
                &pos,
                &sample_options,
            );

            if density > SURFACE_DENSITY_CUTOFF {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        low
    }
}

/// Packs X and Z coordinates into a single i64 for caching.
#[inline]
fn pack_xz(x: i32, z: i32) -> i64 {
    (i64::from(x) & 0xFFFF_FFFF) | ((i64::from(z) & 0xFFFF_FFFF) << 32)
}
