//! Ore vein sampler for large ore vein generation.
//!
//! This module handles the generation of large ore veins (copper and iron)
//! that are placed during the noise generation phase.

use crate::noise::clamped_map;
use crate::random::{PositionalRandom, Random, RandomSplitter};
use crate::BlockStateId;

use super::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use super::chunk_noise_router::ChunkNoiseRouter;
use super::density_function::NoisePos;

/// Configuration for a vein type (copper or iron).
struct VeinType {
    /// Regular ore block.
    ore: BlockStateId,
    /// Raw ore block (rare variant).
    raw_ore: BlockStateId,
    /// Stone type for this depth (granite or tuff).
    stone: BlockStateId,
    /// Minimum Y level for this vein type.
    min_y: i32,
    /// Maximum Y level for this vein type.
    max_y: i32,
}

/// Block state IDs needed by the ore sampler.
#[derive(Clone)]
pub struct OreBlocks {
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

/// Ore vein sampler for copper and iron veins.
pub struct OreVeinSampler {
    random_deriver: RandomSplitter,
    blocks: OreBlocks,
}

impl OreVeinSampler {
    /// Creates a new ore vein sampler.
    #[must_use]
    pub fn new(random_deriver: RandomSplitter, blocks: OreBlocks) -> Self {
        Self {
            random_deriver,
            blocks,
        }
    }

    /// Samples the ore vein at a position.
    ///
    /// Returns `Some(block)` if an ore or stone should be placed,
    /// or `None` if no vein exists at this position.
    pub fn sample(
        &self,
        router: &mut ChunkNoiseRouter,
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> Option<BlockStateId> {
        let block_y = pos.y();

        // Sample vein_toggle to determine vein type
        let vein_toggle = router.vein_toggle(pos, sample_options);

        // Determine vein type based on toggle value
        let vein_type = if vein_toggle > 0.0 {
            // Copper vein
            VeinType {
                ore: if block_y < 0 {
                    self.blocks.deepslate_copper_ore
                } else {
                    self.blocks.copper_ore
                },
                raw_ore: self.blocks.raw_copper_block,
                stone: self.blocks.granite,
                min_y: 0,
                max_y: 50,
            }
        } else {
            // Iron vein
            VeinType {
                ore: if block_y < 0 {
                    self.blocks.deepslate_iron_ore
                } else {
                    self.blocks.iron_ore
                },
                raw_ore: self.blocks.raw_iron_block,
                stone: self.blocks.tuff,
                min_y: -60,
                max_y: -8,
            }
        };

        // Check if we're in the Y range for this vein type
        if block_y < vein_type.min_y || block_y > vein_type.max_y {
            return None;
        }

        // Calculate boundary fade (20 blocks at each boundary)
        let dist_to_min = (block_y - vein_type.min_y) as f64;
        let dist_to_max = (vein_type.max_y - block_y) as f64;
        let boundary_dist = dist_to_min.min(dist_to_max);
        let boundary_fade = clamped_map(boundary_dist, 0.0, 20.0, -0.2, 0.0);

        // Check if we're in a vein core
        let abs_toggle = vein_toggle.abs();
        if abs_toggle + boundary_fade < 0.4 {
            return None;
        }

        // Create position-based random
        let mut random = self.random_deriver.at(pos.x(), block_y, pos.z());

        // Check vein_ridged condition (70% chance requirement)
        if random.next_f32() > 0.7 {
            return None;
        }

        let vein_ridged = router.vein_ridged(pos, sample_options);
        if vein_ridged >= 0.0 {
            return None;
        }

        // Clamp and map the sample value
        let clamped_sample = clamped_map(abs_toggle, 0.4, 0.6, 0.1, 0.3) as f32;

        // Final random check and gap check
        if random.next_f32() < clamped_sample {
            let vein_gap = router.vein_gap(pos, sample_options);
            if vein_gap > -0.3 {
                // 2% chance for raw ore block, 98% for regular ore
                return if random.next_f32() < 0.02 {
                    Some(vein_type.raw_ore)
                } else {
                    Some(vein_type.ore)
                };
            }
        }

        // Return stone if we're in a vein but didn't place ore
        if abs_toggle + boundary_fade >= 0.4 {
            Some(vein_type.stone)
        } else {
            None
        }
    }
}
