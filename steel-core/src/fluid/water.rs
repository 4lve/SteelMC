//! Water fluid implementation.
//!
//! Based on vanilla's WaterFluid.java.

use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_utils::BlockPos;
use steel_utils::types::UpdateFlags;

use crate::world::World;

use super::flowing::{
    FluidBehaviour, FluidState, FluidType, 
    get_new_liquid, get_spread,
    fluid_state_to_block, get_fluid_state,
    is_hole,
};

/// Water fluid behavior.
pub struct WaterFluid;

impl WaterFluid {
    /// Checks if water can spread down to the below position.
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
        if below_fluid.fluid_type == FluidType::Water && !below_fluid.is_source() {
            return true;
        }

        false
    }

    /// Spreads water downward.
    fn spread_down(&self, world: &World, pos: BlockPos, current_tick: u64) -> bool {
        let below = pos.offset(0, -1, 0);
        
        if !self.can_spread_down(world, &pos) {
            return false;
        }

        // Calculate the correct fluid state for the below position
        let new_fluid = get_new_liquid(world, below, FluidType::Water);
        
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
            if fluid.fluid_type == FluidType::Water && fluid.is_source() {
                count += 1;
            }
        }
        
        count
    }

    /// Spreads water to sides using vanilla's algorithm.
    fn spread_to_sides(&self, world: &World, pos: BlockPos, fluid_state: FluidState, current_tick: u64) {
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
        let spreads = get_spread(world, pos, FluidType::Water);
        
        for (direction, new_fluid) in spreads {
            let neighbor = direction.relative(&pos);
            
            // Check if we can actually place there
            let neighbor_state = world.get_block_state(&neighbor);
            let neighbor_block = neighbor_state.get_block();
            
            if !neighbor_block.config.is_air && !neighbor_block.config.replaceable {
                continue;
            }
            
            // Check existing fluid - don't overwrite higher amount water
            let existing = get_fluid_state(world, &neighbor);
            if existing.fluid_type == FluidType::Water && existing.amount() >= new_fluid.amount() {
                continue;
            }
            
            let block_state = fluid_state_to_block(new_fluid);
            
            if world.set_block(neighbor, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE) {
                world.schedule_fluid_tick(neighbor, current_tick, self.tick_delay());
            }
        }
    }
}

impl FluidBehaviour for WaterFluid {
    fn fluid_type(&self) -> FluidType {
        FluidType::Water
    }

    fn tick(&self, world: &World, pos: BlockPos, current_tick: u64) {
        let current_fluid = get_fluid_state(world, &pos);
        
        if current_fluid.is_empty() || current_fluid.fluid_type != FluidType::Water {
            return; // No water here anymore
        }

        // For flowing water, recalculate if it should still exist
        if !current_fluid.is_source() {
            let new_fluid = get_new_liquid(world, pos, FluidType::Water);
            
            if new_fluid.is_empty() {
                // No support - remove the water
                // Note: set_block will trigger neighbor fluid ticks via the world logic
                let air = fluid_state_to_block(FluidState::empty());
                world.set_block(pos, air, UpdateFlags::UPDATE_ALL_IMMEDIATE);
                return;
            } else if new_fluid != current_fluid {
                // Update to new state
                let block_state = fluid_state_to_block(new_fluid);
                world.set_block(pos, block_state, UpdateFlags::UPDATE_ALL_IMMEDIATE);
                
                // If water is shrinking, re-schedule self to continue checking
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

        // Vanilla spread() logic:
        // 1. Try to spread down
        // 2. If can spread down AND has 3+ source neighbors, also spread to sides
        // 3. Otherwise if source OR not a water hole below, spread to sides
        
        let can_spread_down = self.can_spread_down(world, &pos);
        
        if can_spread_down {
            // Try to spread down
            let did_spread_down = self.spread_down(world, pos, current_tick);
            
            if did_spread_down {
                // If we have 3+ source neighbors, also spread to sides (source duplication)
                if self.source_neighbor_count(world, &pos) >= 3 {
                    self.spread_to_sides(world, pos, fluid_state, current_tick);
                }
                return;
            }
        }
        
        // If source OR not a water hole below, spread to sides
        let is_water_hole = is_hole(world, &pos, FluidType::Water);
        
        if fluid_state.is_source() || !is_water_hole {
            self.spread_to_sides(world, pos, fluid_state, current_tick);
        }
    }
}
