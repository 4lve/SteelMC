//! Liquid block behavior (water, lava).
//!
//! Based on vanilla's LiquidBlock.java.
//!
// TODO: Consider moving this to steel-core/src/fluid/block.rs for consistency
//       (fluid logic should be grouped together)
// TODO: Add support for cached fluid states when FluidState caching is implemented
// TODO: Fix deadlock when calling should_spread_liquid from on_place
//       set_block holds chunk locks when calling on_place, and should_spread_liquid
//       needs to read neighbor block states which requires the same locks -> DEADLOCK

use steel_registry::REGISTRY;
use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::Direction;
use steel_registry::vanilla_blocks;
use steel_utils::BlockPos;
use steel_utils::BlockStateId;
use steel_utils::types::UpdateFlags;

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;
use crate::fluid::{get_fluid_state, is_water_state};
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

    /// Checks if this liquid should spread and handles lava-water interactions.
    /// Based on vanilla's LiquidBlock.shouldSpreadLiquid().
    ///
    /// Returns `true` if the liquid should spread (schedule tick),
    /// Returns `false` if the liquid was converted to a block (obsidian/cobblestone/basalt).
    ///
    /// WARNING: This method CANNOT be called from on_place because set_block holds
    /// chunk locks and this method needs to read neighbor blocks -> DEADLOCK
    fn should_spread_liquid(&self, world: &World, pos: BlockPos, _state: BlockStateId) -> bool {
        // Only lava has special interactions with water and blue ice
        if !std::ptr::eq(self.block, vanilla_blocks::LAVA) {
            return true;
        }

        // Check if there's soul soil below (for basalt generation)
        let below_pos = pos.offset(0, -1, 0);
        let below_state = world.get_block_state(&below_pos);
        let has_soul_soil_below = std::ptr::eq(below_state.get_block(), vanilla_blocks::SOUL_SOIL);

        // Get fluid state to check if this is a source
        let fluid_state = get_fluid_state(world, &pos);

        // Check all 4 horizontal directions for water or blue ice
        for direction in [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            let neighbor_pos = direction.opposite().relative(&pos);
            let neighbor_fluid = get_fluid_state(world, &neighbor_pos);

            // Check for water (including flowing_water and waterlogged blocks)
            // Using fluid tag check to support modded fluids registered in the water tag
            if is_water_state(neighbor_fluid) {
                // Lava + Water = Obsidian (if source) or Cobblestone (if flowing)
                let new_block = if fluid_state.is_source() {
                    vanilla_blocks::OBSIDIAN
                } else {
                    vanilla_blocks::COBBLESTONE
                };

                let new_state = REGISTRY.blocks.get_default_state_id(new_block);
                world.set_block(pos, new_state, UpdateFlags::UPDATE_IMMEDIATE);
                return false; // Don't schedule fluid tick - block was converted
            }

            // Check for basalt generation: soul soil below + blue ice adjacent
            if has_soul_soil_below {
                let neighbor_state = world.get_block_state(&neighbor_pos);
                if std::ptr::eq(neighbor_state.get_block(), vanilla_blocks::BLUE_ICE) {
                    let new_state = REGISTRY.blocks.get_default_state_id(vanilla_blocks::BASALT);
                    world.set_block(pos, new_state, UpdateFlags::UPDATE_IMMEDIATE);
                    return false; // Don't schedule fluid tick - block was converted
                }
            }
        }

        true // No interaction occurred, proceed with normal fluid tick
    }
}

impl BlockBehaviour for LiquidBlockBehavior {
    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.block.default_state())
    }

    /// Called when the block is placed.
    fn on_place(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _old_state: BlockStateId,
        _moved_by_piston: bool,
    ) {
        // DEADLOCK WARNING: Calling should_spread_liquid here causes deadlock
        // because set_block holds chunk locks when calling on_place, and
        // should_spread_liquid needs to read neighbor block states.
        //
        // If you uncomment the line below, placing lava will deadlock the server:
        // if self.should_spread_liquid(world, pos, state) {
        //     world.schedule_fluid_tick(pos, world.game_time(), self.tick_delay);
        // }
        //
        // For now, we just schedule the tick and let the fluid behavior handle
        // the interaction check when the tick fires (outside of set_block locks).
        world.schedule_fluid_tick(pos, world.game_time(), self.tick_delay);
    }

    /// Called when a neighboring block changes.
    ///
    /// This is safe to call should_spread_liquid here because handle_neighbor_changed
    /// is called outside of set_block locks (after the block is already placed).
    fn handle_neighbor_changed(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _source_block: BlockRef,
        _moved_by_piston: bool,
    ) {
        // This is safe because we're not inside set_block locks
        if self.should_spread_liquid(world, pos, state) {
            world.schedule_fluid_tick(pos, world.game_time(), self.tick_delay);
        }
    }

    /// Called when a neighbor's shape changes.
    fn update_shape(
        &self,
        state: BlockStateId,
        _world: &World,
        _pos: BlockPos,
        _direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        state
    }
}
