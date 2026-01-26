//! Liquid block behavior (water, lava).
//!
//! Based on vanilla's LiquidBlock.java.

use steel_registry::blocks::BlockRef;
use steel_utils::BlockPos;
use steel_utils::BlockStateId;

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;
use crate::fluid::{FluidType, get_fluid_state};
use crate::world::World;

/// Behavior for liquid blocks (water, lava).
/// 
/// Key behavior: when a neighbor changes, schedule a tick for this block
/// so it can recalculate and potentially de-propagate.
pub struct LiquidBlockBehavior {
    block: BlockRef,
    fluid_type: FluidType,
}

impl LiquidBlockBehavior {
    /// Creates a new liquid block behavior.
    #[must_use]
    pub const fn new(block: BlockRef, fluid_type: FluidType) -> Self {
        Self { block, fluid_type }
    }
}

impl BlockBehaviour for LiquidBlockBehavior {
    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.block.default_state())
    }

    /// Called when a neighboring block changes.
    /// 
    /// This is the key to de-propagation: when a neighbor is removed/changed,
    /// we schedule a tick for this block so it can recalculate whether it
    /// should still exist.
    fn handle_neighbor_changed(
        &self,
        _state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _source_block: BlockRef,
        _moved_by_piston: bool,
    ) {
        // Schedule a tick for this block to recalculate
        // Neighbors get notified and schedule their own ticks
        let tick_delay = self.fluid_type.tick_delay();
        world.schedule_fluid_tick(pos, world.game_time(), tick_delay);
    }

    /// Called when a neighbor's shape changes.
    /// Note: vanilla's LiquidBlock.updateShape only schedules if current OR neighbor is source.
    /// Let's be more conservative to avoid double-scheduling.
    fn update_shape(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _direction: steel_registry::blocks::properties::Direction,
        _neighbor_pos: BlockPos,
        neighbor_state: BlockStateId,
    ) -> BlockStateId {
        // Note: Vanilla calls scheduleTick here if source, but since set_block triggers 
        // handle_neighbor_changed which ALSO schedules, this is redundant in our implementation
        // and causes double logging/scheduling.
        
        state
    }
}

