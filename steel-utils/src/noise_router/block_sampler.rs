//! Block state sampler chain for terrain generation.
//!
//! This module provides a chained sampler that combines aquifer and ore vein
//! sampling to determine the final block state at each position.

use crate::BlockStateId;

use super::aquifer_sampler::{AquiferSampler, AquiferSamplerImpl};
use super::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use super::chunk_noise_router::ChunkNoiseRouter;
use super::density_function::NoisePos;
use super::ore_sampler::OreVeinSampler;
use super::surface_height_sampler::SurfaceHeightEstimateSampler;

/// A single block state sampler.
pub enum BlockStateSampler {
    /// Aquifer sampler for water/lava placement.
    Aquifer(AquiferSampler),
    /// Ore vein sampler for large ore veins.
    Ore(OreVeinSampler),
}

impl BlockStateSampler {
    /// Samples the block state at a position.
    pub fn sample(
        &mut self,
        router: &mut ChunkNoiseRouter,
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> Option<BlockStateId> {
        match self {
            Self::Aquifer(aquifer) => aquifer.apply(router, pos, sample_options, height_estimator),
            Self::Ore(ore) => ore.sample(router, pos, sample_options),
        }
    }
}

/// A chained block state sampler that tries multiple samplers in order.
pub struct ChainedBlockStateSampler {
    samplers: Box<[BlockStateSampler]>,
}

impl ChainedBlockStateSampler {
    /// Creates a new chained block state sampler.
    #[must_use]
    pub fn new(samplers: Box<[BlockStateSampler]>) -> Self {
        Self { samplers }
    }

    /// Creates a sampler with just an aquifer (no ore veins).
    #[must_use]
    pub fn aquifer_only(aquifer: AquiferSampler) -> Self {
        Self {
            samplers: Box::new([BlockStateSampler::Aquifer(aquifer)]),
        }
    }

    /// Creates a sampler with aquifer and ore veins.
    #[must_use]
    pub fn with_ores(aquifer: AquiferSampler, ore: OreVeinSampler) -> Self {
        Self {
            samplers: Box::new([
                BlockStateSampler::Aquifer(aquifer),
                BlockStateSampler::Ore(ore),
            ]),
        }
    }

    /// Samples the block state at a position.
    ///
    /// Tries each sampler in order until one returns a block state.
    /// Returns `None` if no sampler determined a specific block (use default solid).
    pub fn sample(
        &mut self,
        router: &mut ChunkNoiseRouter,
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
        height_estimator: &mut SurfaceHeightEstimateSampler,
    ) -> Option<BlockStateId> {
        for sampler in self.samplers.iter_mut() {
            if let Some(state) = sampler.sample(router, pos, sample_options, height_estimator) {
                return Some(state);
            }
        }
        None
    }
}
