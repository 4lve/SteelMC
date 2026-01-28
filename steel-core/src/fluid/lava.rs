//! Lava fluid implementation.
//!
//! Based on vanilla's LavaFluid.java.
//!
// TODO: Consider extracting common fluid behavior into a macro or generic implementation
//       to reduce duplication between WaterFluid and LavaFluid
// TODO: Add doc comments for all private helper methods
// TODO: Consider moving can_spread_down/spread_down to a shared trait when more fluids are added

use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::Direction;
use steel_utils::types::UpdateFlags;
use steel_utils::BlockPos;

use crate::world::World;

use super::flowing::{
    can_hold_any_fluid, fluid_state_to_block, get_fluid_state, get_new_liquid, get_spread, is_hole,
    FluidBehaviour, FluidState,
};
use steel_registry::FluidId;

/// Lava fluid behavior.
pub struct LavaFluid;

impl LavaFluid {
    /// Checks if lava can spread down to the below position.
    fn can_spread_down(&self, world: &World, pos: &BlockPos) -> bool {
        let below = pos.offset(0, -1, 0);

        if !world.is_in_valid_bounds(&below) {
            return false;
        }

        let below_state = world.get_block_state(&below);
        let below_block = below_state.get_block();

        // Can flow into air or replaceable
        if below_block.config.is_air || below_block.config.replaceable {
            return true;
        }

        // Can flow into same fluid type
        let below_fluid = get_fluid_state(world, &below);
        if below_fluid.fluid == FluidId::Lava && !below_fluid.is_source() {
            return true;
        }

        false
    }

    /// Spreads lava downward.
    fn spread_down(
        &self,
        world: &World,
        pos: BlockPos,
        fluid_state: FluidState,
        current_tick: u64,
    ) -> bool {
        let below = pos.offset(0, -1, 0);

        if !world.is_in_valid_bounds(&below) {
            return false;
        }

        let below_state = world.get_block_state(&below);
        let below_block = below_state.get_block();
        let below_fluid = get_fluid_state(world, &below);

        // Lava-water interaction: lava flowing down into water
        if below_fluid.fluid == FluidId::Water {
            use steel_registry::vanilla_blocks;
            use steel_registry::REGISTRY;

            // If lava is source -> obsidian, otherwise -> cobblestone
            let is_lava_source = fluid_state.is_source();
            let new_block = if is_lava_source {
                vanilla_blocks::OBSIDIAN
            } else {
                vanilla_blocks::COBBLESTONE
            };

            let block_state = REGISTRY.blocks.get_default_state_id(new_block);
            world.set_block(below, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE);
            // TODO: Play fizz sound effect (level event 1501)
            return true;
        }

        if !self.can_spread_down(world, &pos) {
            return false;
        }

        // Calculate the correct fluid state for the below position
        let new_fluid = get_new_liquid(world, below, FluidId::Lava, self.drop_off());

        if new_fluid.is_empty() {
            return false;
        }

        let block_state = fluid_state_to_block(new_fluid);

        if world.set_block(below, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE) {
            world.schedule_fluid_tick(below, current_tick, self.tick_delay());
            return true;
        }

        false
    }

    /// Counts adjacent source blocks.
    fn source_neighbor_count(&self, world: &World, pos: &BlockPos) -> u8 {
        let mut count = 0u8;

        for offset in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)] {
            let neighbor = pos.offset(offset.0, offset.1, offset.2);
            let fluid = get_fluid_state(world, &neighbor);
            if fluid.fluid == FluidId::Lava && fluid.is_source() {
                count += 1;
            }
        }

        count
    }

    /// Spreads lava to sides using vanilla's algorithm.
    fn spread_to_sides(
        &self,
        world: &World,
        pos: BlockPos,
        fluid_state: FluidState,
        current_tick: u64,
        slope_find_distance: u8,
    ) {
        // NOTE: The lava-water interaction for adjacent blocks (cobblestone generator)
        // is now handled in LiquidBlockBehavior.on_place and handle_neighbor_changed
        // to match vanilla's shouldSpreadLiquid logic.

        // Calculate spread amount - vanilla: fluidState.getAmount() - dropOff
        // Or 7 if falling (like level 1)
        let new_amount = if fluid_state.falling {
            7 // Falling water spreads at amount 7 (= level 1)
        } else {
            fluid_state.amount().saturating_sub(1)
        };

        if new_amount == 0 {
            return; // No more water to spread
        }

        // Get spread map using slope finding
        let spreads = get_spread(
            world,
            pos,
            FluidId::Lava,
            self.drop_off(),
            slope_find_distance,
        );

        for (direction, new_fluid) in spreads {
            let neighbor = direction.relative(&pos);

            // Check if the position can hold fluid
            if !can_hold_any_fluid(world, &neighbor) {
                continue;
            }

            // Check existing fluid
            let existing = get_fluid_state(world, &neighbor);

            // Lava-water interaction: lava flowing INTO water creates obsidian/cobblestone
            // This is different from adjacent interaction - this is when lava spreads into water
            if existing.fluid == FluidId::Water {
                use steel_registry::vanilla_blocks;
                use steel_registry::REGISTRY;

                // If lava is source -> obsidian, otherwise -> cobblestone
                let is_lava_source = fluid_state.is_source();
                let new_block = if is_lava_source {
                    vanilla_blocks::OBSIDIAN
                } else {
                    vanilla_blocks::COBBLESTONE
                };

                let block_state = REGISTRY.blocks.get_default_state_id(new_block);
                world.set_block(neighbor, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE);
                // TODO: Play fizz sound effect (level event 1501)
                continue;
            }

            // Check if existing fluid can be replaced
            if !existing.is_empty() {
                // Don't overwrite higher amount of same fluid type
                // For same fluid, we allow replacement if the new amount is higher
                // (this allows lava to "level up" as more sources contribute)
                if existing.fluid == FluidId::Lava {
                    if existing.amount() >= new_fluid.amount() {
                        continue;
                    }
                    // Otherwise, allow replacement - lava can flow into lower-level lava
                } else if existing.fluid == FluidId::Water {
                    // For water, check if lava can replace it
                    // Lava can replace water if lava height >= 0.44444445
                    if !(fluid_state.amount() as f32 / 9.0 >= 0.44444445) {
                        continue;
                    }
                    // Otherwise, obsidian/cobblestone will be created
                } else {
                    // For other fluids/empty, check can_be_replaced_with
                    if !self.can_be_replaced_with(
                        existing,
                        world,
                        neighbor,
                        FluidId::Lava,
                        direction,
                    ) {
                        continue;
                    }
                }
            }

            let block_state = fluid_state_to_block(new_fluid);

            if world.set_block(neighbor, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE) {
                world.schedule_fluid_tick(neighbor, current_tick, self.tick_delay());
            }
        }
    }
}

impl FluidBehaviour for LavaFluid {
    fn fluid_type(&self) -> FluidId {
        FluidId::Lava
    }

    fn tick_delay(&self) -> u32 {
        30
    }

    fn drop_off(&self) -> u8 {
        2
    }

    fn slope_find_distance(&self) -> u8 {
        2
    }

    fn tick(&self, world: &World, pos: BlockPos, current_tick: u64) {
        let tick_delay = 30;

        let current_fluid = get_fluid_state(world, &pos);

        if current_fluid.is_empty() || current_fluid.fluid != FluidId::Lava {
            return; // No lava here anymore
        }

        // For flowing lava, recalculate if it should still exist
        if !current_fluid.is_source() {
            let new_fluid = get_new_liquid(world, pos, FluidId::Lava, self.drop_off());

            if new_fluid.is_empty() {
                // No support - remove the lava
                // Note: set_block will trigger neighbor fluid ticks via the world logic
                let air = fluid_state_to_block(FluidState::empty());
                world.set_block(pos, air, UpdateFlags::UPDATE_ALL_IMMEDIATE);
                return;
            }

            if new_fluid != current_fluid {
                // Update to new state
                let block_state = fluid_state_to_block(new_fluid);
                world.set_block(pos, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE);

                // If lava is shrinking, re-schedule self to continue checking
                // Don't schedule all neighbors - let natural tick propagation handle it
                if new_fluid.amount() < current_fluid.amount() {
                    world.schedule_fluid_tick(pos, current_tick, self.tick_delay());
                    return; // Don't spread when shrinking
                }
            }
        }

        // Spread using vanilla's algorithm
        self.spread(world, pos, current_fluid, current_tick);
    }

    fn spread(&self, world: &World, pos: BlockPos, fluid_state: FluidState, current_tick: u64) {
        if fluid_state.is_empty() {
            return;
        }

        let slope_find_distance = 2;

        // Vanilla spread() logic:
        // 1. Try to spread down
        // 2. If can spread down AND has 3+ source neighbors, also spread to sides
        // 3. Otherwise if source OR not a water hole below, spread to sides

        let can_spread_down = self.can_spread_down(world, &pos);

        if can_spread_down {
            // Try to spread down
            let did_spread_down = self.spread_down(world, pos, fluid_state, current_tick);

            if did_spread_down {
                // If we have 3+ source neighbors, also spread to sides (source duplication)
                if self.source_neighbor_count(world, &pos) >= 3 {
                    self.spread_to_sides(
                        world,
                        pos,
                        fluid_state,
                        current_tick,
                        slope_find_distance,
                    );
                }
                return;
            }
        }

        // If source OR not a lava hole below, spread to sides
        let is_lava_hole = is_hole(world, &pos, FluidId::Lava);

        if fluid_state.is_source() || !is_lava_hole {
            self.spread_to_sides(world, pos, fluid_state, current_tick, slope_find_distance);
        }
    }

    /// Returns true if lava can be replaced by another fluid.
    /// Based on vanilla LavaFluid.canBeReplacedWith().
    /// Lava can be replaced if its height >= 0.44444445F and the fluid is water.
    fn can_be_replaced_with(
        &self,
        fluid_state: FluidState,
        _world: &World,
        _pos: BlockPos,
        other_fluid: FluidId,
        _direction: Direction,
    ) -> bool {
        // Lava can be replaced if its height >= 0.44444445F (4/9 of a block)
        // and the replacing fluid is water
        let height = fluid_state.amount() as f32 / 9.0; // Convert amount to height (0-1)
        height >= 0.44444445 && other_fluid == FluidId::Water
    }
}
