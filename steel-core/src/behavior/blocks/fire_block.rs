//! Fire block behavior.
//!
//! Fire spreads to nearby flammable blocks and can burn them.
//! Uses scheduled ticks (every 30-40 ticks) rather than random ticks.

use std::sync::Arc;

use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{BlockStateProperties, Direction};
use steel_registry::level_events;
use steel_registry::vanilla_blocks;
use steel_utils::types::UpdateFlags;
use steel_utils::{BlockPos, BlockStateId};

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;
use crate::player::Player;
use crate::world::World;

/// Fire tick delay: 30 + random(0..10) ticks.
const FIRE_TICK_DELAY_MIN: u32 = 30;
const FIRE_TICK_DELAY_RANGE: u32 = 10;

/// Behavior for fire blocks.
///
/// Fire uses scheduled ticks to:
/// - Age up over time
/// - Spread to nearby flammable blocks
/// - Burn adjacent flammable blocks
/// - Extinguish if conditions aren't met
pub struct FireBlock {
    block: BlockRef,
}

impl FireBlock {
    /// Creates a new fire block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Gets the age of the fire from its block state.
    fn get_age(state: BlockStateId) -> u8 {
        state.get_value(&BlockStateProperties::AGE_15)
    }

    /// Returns the fire state with the given age and directional properties.
    fn get_state_with_age(&self, world: &World, pos: BlockPos, age: u8) -> BlockStateId {
        let base_state = self.get_fire_state(world, pos);
        base_state.set_value(&BlockStateProperties::AGE_15, age)
    }

    /// Calculates the fire state based on surrounding blocks.
    /// Sets directional flags (NORTH, SOUTH, EAST, WEST, UP) based on
    /// whether there's a flammable block in that direction.
    fn get_fire_state(&self, world: &World, pos: BlockPos) -> BlockStateId {
        let below = pos.offset(0, -1, 0);
        let below_state = world.get_block_state(&below);

        // If there's a solid block below, use floor fire (no directional flags)
        if below_state.is_face_sturdy(Direction::Up) {
            return self.block.default_state();
        }

        // Otherwise, set flags based on adjacent flammable blocks
        let mut state = self.block.default_state();

        if Self::can_burn(world.get_block_state(&pos.offset(0, 0, -1))) {
            state = state.set_value(&BlockStateProperties::NORTH, true);
        }
        if Self::can_burn(world.get_block_state(&pos.offset(0, 0, 1))) {
            state = state.set_value(&BlockStateProperties::SOUTH, true);
        }
        if Self::can_burn(world.get_block_state(&pos.offset(-1, 0, 0))) {
            state = state.set_value(&BlockStateProperties::WEST, true);
        }
        if Self::can_burn(world.get_block_state(&pos.offset(1, 0, 0))) {
            state = state.set_value(&BlockStateProperties::EAST, true);
        }
        if Self::can_burn(world.get_block_state(&pos.offset(0, 1, 0))) {
            state = state.set_value(&BlockStateProperties::UP, true);
        }

        state
    }

    /// Returns true if a block can catch fire.
    fn can_burn(state: BlockStateId) -> bool {
        Self::get_ignite_odds_for_state(state) > 0
    }

    /// Gets the ignite odds for a block state.
    /// Returns 0 for waterlogged blocks.
    fn get_ignite_odds_for_state(state: BlockStateId) -> u8 {
        // Waterlogged blocks don't burn
        if let Some(true) = state.try_get_value(&BlockStateProperties::WATERLOGGED) {
            return 0;
        }
        state.get_block().config.ignite_odds
    }

    /// Gets the burn odds for a block state.
    /// Returns 0 for waterlogged blocks.
    fn get_burn_odds_for_state(state: BlockStateId) -> u8 {
        // Waterlogged blocks don't burn
        if let Some(true) = state.try_get_value(&BlockStateProperties::WATERLOGGED) {
            return 0;
        }
        state.get_block().config.burn_odds
    }

    /// Gets the maximum ignite odds of blocks adjacent to the given position.
    /// Only considers the position if it's an air block.
    fn get_ignite_odds_at(world: &World, pos: BlockPos) -> u8 {
        let state = world.get_block_state(&pos);
        if !state.get_block().config.is_air {
            return 0;
        }

        let mut max_odds = 0u8;
        for direction in Direction::VALUES {
            let neighbor_pos = direction.relative(&pos);
            let neighbor_state = world.get_block_state(&neighbor_pos);
            max_odds = max_odds.max(Self::get_ignite_odds_for_state(neighbor_state));
        }
        max_odds
    }

    /// Checks if there's a valid fire location (adjacent flammable block).
    fn is_valid_fire_location(world: &World, pos: BlockPos) -> bool {
        for direction in Direction::VALUES {
            let neighbor_pos = direction.relative(&pos);
            if Self::can_burn(world.get_block_state(&neighbor_pos)) {
                return true;
            }
        }
        false
    }

    /// Checks if fire can survive at this position.
    /// Fire survives if there's a solid block below OR an adjacent flammable block.
    fn can_survive(world: &World, pos: BlockPos) -> bool {
        let below = pos.offset(0, -1, 0);
        let below_state = world.get_block_state(&below);
        below_state.is_face_sturdy(Direction::Up) || Self::is_valid_fire_location(world, pos)
    }

    /// Tries to burn a block, potentially replacing it with fire or removing it.
    fn check_burn_out(&self, world: &World, pos: BlockPos, chance: u32, age: u8) {
        let state = world.get_block_state(&pos);
        let burn_odds = u32::from(Self::get_burn_odds_for_state(state));

        if rand::random::<u32>() % chance < burn_odds {
            // TODO: Check if raining at position
            if rand::random::<u32>() % (u32::from(age) + 10) < 5 {
                // Replace with fire
                let new_age = (age + rand::random::<u8>() % 5 / 4).min(15);
                let fire_state = self.get_state_with_age(world, pos, new_age);
                world.set_block(pos, fire_state, UpdateFlags::UPDATE_ALL);
            } else {
                // Destroy the block
                world.set_block(
                    pos,
                    vanilla_blocks::AIR.default_state(),
                    UpdateFlags::UPDATE_ALL,
                );
            }
        }
    }

    /// Returns a random fire tick delay (30-39 ticks).
    fn get_fire_tick_delay() -> u32 {
        FIRE_TICK_DELAY_MIN + rand::random::<u32>() % FIRE_TICK_DELAY_RANGE
    }
}

impl BlockBehaviour for FireBlock {
    fn update_shape(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        // Check if fire can survive here
        if !Self::can_survive(world, pos) {
            return vanilla_blocks::AIR.default_state();
        }

        // Recalculate fire state with current age
        let age = Self::get_age(state);
        self.get_state_with_age(world, pos, age)
    }

    fn player_will_destroy(
        &self,
        _state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _player: &Player,
    ) -> bool {
        // Play fire extinguish sound instead of block break sound
        world.level_event(level_events::SOUND_EXTINGUISH_FIRE, pos, 0, None);
        true // We handled the effect, skip default
    }

    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.get_fire_state(context.world, context.relative_pos))
    }

    fn on_place(
        &self,
        _state: BlockStateId,
        world: &World,
        pos: BlockPos,
        _old_state: BlockStateId,
        _moved_by_piston: bool,
    ) {
        // Schedule the first fire tick
        world.schedule_tick(pos, self.block, Self::get_fire_tick_delay());
    }

    fn scheduled_tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        // Schedule next tick
        world.schedule_tick(pos, self.block, Self::get_fire_tick_delay());

        // TODO: Check if world allows fire spread (canSpreadFireAround)
        // For now, always allow

        // Check if fire can survive here
        let below = pos.offset(0, -1, 0);
        let below_state = world.get_block_state(&below);
        let is_on_solid = below_state.is_face_sturdy(Direction::Up);

        // TODO: Check for infiniburn tag on block below
        let infini_burn = false;

        let age = Self::get_age(state);

        // TODO: Handle rain extinguishing fire
        // if !infini_burn && is_raining && near_rain {
        //     if rand < 0.2 + age * 0.03 { remove fire }
        // }

        // Age up the fire
        let new_age = (age + rand::random::<u8>() % 3 / 2).min(15);
        if age != new_age {
            let new_state = state.set_value(&BlockStateProperties::AGE_15, new_age);
            // Flag 260 = UPDATE_CLIENTS | UPDATE_KNOWN_SHAPE (4 + 256)
            world.set_block(pos, new_state, UpdateFlags::UPDATE_CLIENTS);
        }

        // Check if fire should extinguish
        if !infini_burn {
            if !Self::is_valid_fire_location(world, pos) {
                if !is_on_solid || age > 3 {
                    world.set_block(
                        pos,
                        vanilla_blocks::AIR.default_state(),
                        UpdateFlags::UPDATE_ALL,
                    );
                }
                return;
            }

            // At max age, chance to extinguish if not on flammable block
            if age == 15 && rand::random::<u32>().is_multiple_of(4) && !Self::can_burn(below_state)
            {
                world.set_block(
                    pos,
                    vanilla_blocks::AIR.default_state(),
                    UpdateFlags::UPDATE_ALL,
                );
                return;
            }
        }

        // TODO: Check increased fire burnout environment attribute
        let extra = 0i32;

        // Try to burn adjacent blocks
        self.check_burn_out(world, pos.offset(1, 0, 0), (300 + extra) as u32, age); // east
        self.check_burn_out(world, pos.offset(-1, 0, 0), (300 + extra) as u32, age); // west
        self.check_burn_out(world, pos.offset(0, -1, 0), (250 + extra) as u32, age); // below
        self.check_burn_out(world, pos.offset(0, 1, 0), (250 + extra) as u32, age); // above
        self.check_burn_out(world, pos.offset(0, 0, -1), (300 + extra) as u32, age); // north
        self.check_burn_out(world, pos.offset(0, 0, 1), (300 + extra) as u32, age); // south

        // Try to spread fire to nearby air blocks
        for dx in -1i32..=1 {
            for dz in -1i32..=1 {
                for dy in -1i32..=4 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }

                    let test_pos = pos.offset(dx, dy, dz);
                    let mut rate = 100u32;
                    if dy > 1 {
                        rate += (dy - 1) as u32 * 100;
                    }

                    let ignite_odds = u32::from(Self::get_ignite_odds_at(world, test_pos));
                    if ignite_odds > 0 {
                        // TODO: Get difficulty from world (0-3)
                        let difficulty = 2u32; // Normal difficulty
                        let odds = (ignite_odds + 40 + difficulty * 7) / (u32::from(age) + 30);

                        // TODO: Check increased fire burnout
                        // if increased_burnout { odds /= 2; }

                        // TODO: Check rain
                        if odds > 0 && rand::random::<u32>() % rate <= odds {
                            let spread_age = (age + rand::random::<u8>() % 5 / 4).min(15);
                            let fire_state = self.get_state_with_age(world, test_pos, spread_age);
                            world.set_block(test_pos, fire_state, UpdateFlags::UPDATE_ALL);
                        }
                    }
                }
            }
        }
    }
}
