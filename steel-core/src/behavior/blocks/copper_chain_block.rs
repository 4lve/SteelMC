//! Weathering copper chain block behavior implementation.
//!
//! Copper chains are oriented blocks with an axis property that determines their direction.
//! They can be waterlogged and will weather (oxidize) over time unless waxed.

use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{BlockStateProperties, BoolProperty, EnumProperty};
use steel_utils::BlockStateId;
use steel_utils::math::Axis;

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;

/// Behavior for weathering copper chain blocks.
///
/// Copper chains have an axis property that is set based on which face was clicked
/// during placement, can be waterlogged, and will oxidize over time.
pub struct WeatheringCopperChainBlock {
    block: BlockRef,
}

impl WeatheringCopperChainBlock {
    /// Axis property for the chain orientation.
    pub const AXIS: EnumProperty<Axis> = BlockStateProperties::AXIS;
    /// Waterlogged property.
    pub const WATERLOGGED: BoolProperty = BlockStateProperties::WATERLOGGED;

    /// Creates a new weathering copper chain block behavior for the given block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehaviour for WeatheringCopperChainBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(
            self.block
                .default_state()
                .set_value(&Self::AXIS, context.clicked_face.get_axis())
                .set_value(&Self::WATERLOGGED, context.is_water_source()),
        )
    }
}
