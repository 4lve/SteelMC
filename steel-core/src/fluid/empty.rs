//! Empty fluid implementation.
//!
//! Represents the absence of fluid in a block space.
//!
// TODO: This file is minimal - consider if EmptyFluid needs more methods when modded fluids are added

use steel_registry::blocks::properties::Direction;
use steel_registry::fluid_tags;
use steel_utils::types::BlockPos;

use crate::fluid::FluidBehaviour;
use crate::fluid::FluidState;
use crate::world::World;

/// Empty fluid behavior - represents the absence of fluid.
pub struct EmptyFluid;

impl FluidBehaviour for EmptyFluid {
    fn fluid_type(&self) -> u8 {
        fluid_tags::EMPTY
    }

    fn tick(&self, _world: &World, _pos: BlockPos, _current_tick: u64) {
        // Vanilla: nothing
    }

    fn spread(&self, _world: &World, _pos: BlockPos, _fluid_state: FluidState, _current_tick: u64) {
        // Vanilla: nothing
    }

    fn tick_delay(&self) -> u32 {
        0
    }

    fn drop_off(&self) -> u8 {
        0
    }

    fn slope_find_distance(&self) -> u8 {
        0
    }

    /// Returns true if empty can be replaced by another fluid.
    /// Based on vanilla EmptyFluid.canBeReplacedWith().
    /// Empty can always be replaced.
    fn can_be_replaced_with(
        &self,
        _fluid_state: FluidState,
        _world: &World,
        _pos: BlockPos,
        _other_fluid: u8,
        _direction: Direction,
    ) -> bool {
        // Empty can always be replaced by any fluid
        true
    }
}
