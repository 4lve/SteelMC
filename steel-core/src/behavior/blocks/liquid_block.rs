//! Liquid block behavior (water, lava).
//!
//! Based on vanilla's LiquidBlock.java.
//!
// TODO: Consider moving this to steel-core/src/fluid/block.rs for consistency
//       (fluid logic should be grouped together)
// TODO: Add support for cached fluid states when FluidState caching is implemented
// TODO: Implement lava-water interaction (obsidian/cobblestone generation)
//       This is complex and requires careful handling to avoid deadlocks.
//       Vanilla logic is in LiquidBlock.shouldSpreadLiquid():
//       - Check if lava block is adjacent to water
//       - If yes, convert to obsidian (source) or cobblestone (flowing)
//       - Must NOT schedule fluid tick after conversion

use steel_registry::blocks::BlockRef;
use steel_utils::BlockPos;
use steel_utils::BlockStateId;

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;
use crate::world::World;

/// Behavior for liquid blocks (water, lava).
///
/// Key behavior: when a neighbor changes, schedule a tick for this block
/// so it can recalculate and potentially de-propagate.
pub struct LiquidBlockBehavior {
    block: BlockRef,
    tick_delay: u32,
}

impl LiquidBlockBehavior {
    /// Creates a new liquid block behavior.
    #[must_use]
    pub const fn new(block: BlockRef, tick_delay: u32) -> Self {
        Self { block, tick_delay }
    }
}

impl BlockBehaviour for LiquidBlockBehavior {
    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.block.default_state())
    }

    /// Called when the block is placed.
    fn on_place(
        &self,
        _state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _old_state: BlockStateId,
        _moved_by_piston: bool,
    ) {
        // Schedule tick for fluid spreading
        // TODO: Add lava-water interaction check here before scheduling
        // (see vanilla LiquidBlock.shouldSpreadLiquid)
        world.schedule_fluid_tick(pos, world.game_time(), self.tick_delay);
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
        // TODO: Add lava-water interaction check here
        // (see vanilla LiquidBlock.shouldSpreadLiquid)
        // When water is placed next to lava, lava should convert to obsidian/cobblestone

        // Schedule a tick for this block to recalculate
        world.schedule_fluid_tick(pos, world.game_time(), self.tick_delay);
    }

    /// Called when a neighbor's shape changes.
    fn update_shape(
        &self,
        state: BlockStateId,
        _world: &World,
        _pos: BlockPos,
        _direction: steel_registry::blocks::properties::Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        state
    }
}
