//! Manual block behavior assignments for vanilla blocks.
//!
//! This module is for custom block behaviors that are too complex to auto-generate.
//! Add custom behaviors here for blocks like fences, doors, redstone components, etc.

use crate::{
    blocks::{BlockRegistry, vanilla_behaviours::rotated_pillar_block::RotatedPillarBlock},
    vanilla_blocks,
};

static OAK_LOG: RotatedPillarBlock = RotatedPillarBlock::new(vanilla_blocks::OAK_LOG);

pub fn assign_custom_block_behaviors(registry: &mut BlockRegistry) {
    registry.set_behavior(vanilla_blocks::OAK_LOG, &OAK_LOG);
}
