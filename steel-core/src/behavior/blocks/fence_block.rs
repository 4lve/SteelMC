//! Fence block behavior implementation.
//!
//! Fences connect to adjacent fences, fence gates, and solid blocks.

use steel_registry::REGISTRY;
use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::{BlockStateExt, is_exception_for_connection};
use steel_registry::blocks::properties::{BlockStateProperties, BoolProperty, Direction};
use steel_utils::{BlockPos, BlockStateId, Identifier};

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;
use crate::world::World;

/// Behavior for fence blocks.
///
/// Fences have 4 boolean properties (north, east, south, west) that indicate
/// whether the fence connects in that direction. A fence connects to:
/// - Other fences of the same type
/// - Fence gates facing the appropriate direction
/// - Blocks with a sturdy face on the connecting side
pub struct FenceBlock {
    block: BlockRef,
}

impl FenceBlock {
    /// North connection property.
    pub const NORTH: BoolProperty = BlockStateProperties::NORTH;
    /// East connection property.
    pub const EAST: BoolProperty = BlockStateProperties::EAST;
    /// South connection property.
    pub const SOUTH: BoolProperty = BlockStateProperties::SOUTH;
    /// West connection property.
    pub const WEST: BoolProperty = BlockStateProperties::WEST;
    /// Waterlogged property.
    pub const WATERLOGGED: BoolProperty = BlockStateProperties::WATERLOGGED;

    /// Creates a new fence block behavior for the given block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Checks if this fence should connect to the given neighbor state.
    fn connects_to(&self, neighbor_state: BlockStateId, direction: Direction) -> bool {
        let opposite = direction.opposite();
        if is_exception_for_connection(neighbor_state) {
            return false;
        }
        if neighbor_state.is_face_sturdy(opposite) {
            return true;
        }
        if self.is_same_fence_type(neighbor_state) {
            return true;
        }
        if Self::connect_to_fence_gates(neighbor_state, direction) {
            return true;
        }
        false
    }

    fn get_fence_gates_direction(state: BlockStateId) -> Option<Direction> {
        if let Some(facing_str) = state.get_property_str("facing") {
            let facing = match facing_str.as_str() {
                "north" => Some(Direction::North),
                "south" => Some(Direction::South),
                "east" => Some(Direction::East),
                "west" => Some(Direction::West),
                _ => None,
            };
            return facing;
        }
        None
    }

    fn connect_to_fence_gates(state: BlockStateId, direction: Direction) -> bool {
        // Check if it's a fence gate facing the right direction
        let fence_gates_tag = Identifier::vanilla_static("fence_gates");
        if !REGISTRY
            .blocks
            .is_in_tag(state.get_block(), &fence_gates_tag)
        {
            return false;
        }
        // Fence gates connect perpendicular to their facing direction
        // A gate facing north/south connects to fences to its east/west
        // A gate facing east/west connects to fences to its north/south

        let gate_facing = Self::get_fence_gates_direction(state);
        let Some(gate_facing) = gate_facing else {
            return false;
        };
        // Gate connects perpendicular to its facing
        let connects = match (gate_facing, direction) {
            // Gate facing N/S connects to blocks on E/W sides,
            // Gate facing E/W connects to blocks on N/S sides
            (Direction::North | Direction::South, Direction::East | Direction::West)
            | (Direction::East | Direction::West, Direction::North | Direction::South) => true,
            _ => false,
        };
        if connects {
            return true;
        }
        false
    }

    fn is_same_fence_type(&self, neighbor_state: BlockStateId) -> bool {
        let neighbor_block = neighbor_state.get_block();
        let fences_tag = Identifier::vanilla_static("fences");
        if !REGISTRY.blocks.is_in_tag(neighbor_block, &fences_tag) {
            return false;
        }
        let wooden_fences_tag = Identifier::vanilla_static("wooden_fences");
        REGISTRY.blocks.is_in_tag(self.block, &wooden_fences_tag)
            == REGISTRY
                .blocks
                .is_in_tag(neighbor_block, &wooden_fences_tag)
    }

    /// Gets the connection state for a position by checking all 4 horizontal neighbors.
    fn get_connection_state(&self, world: &World, pos: &BlockPos) -> BlockStateId {
        let mut state = self.block.default_state();

        // Check north
        let north_pos = Direction::North.relative(pos);
        let north_state = world.get_block_state(&north_pos);
        let connects_north = self.connects_to(north_state, Direction::North);
        state = state.set_value(&Self::NORTH, connects_north);

        // Check east
        let east_pos = Direction::East.relative(pos);
        let east_state = world.get_block_state(&east_pos);
        let connects_east = self.connects_to(east_state, Direction::East);
        state = state.set_value(&Self::EAST, connects_east);

        // Check south
        let south_pos = Direction::South.relative(pos);
        let south_state = world.get_block_state(&south_pos);
        let connects_south = self.connects_to(south_state, Direction::South);
        state = state.set_value(&Self::SOUTH, connects_south);

        // Check west
        let west_pos = Direction::West.relative(pos);
        let west_state = world.get_block_state(&west_pos);
        let connects_west = self.connects_to(west_state, Direction::West);
        state = state.set_value(&Self::WEST, connects_west);

        state
    }
}

impl BlockBehaviour for FenceBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        log::debug!(
            "FenceBlock::get_state_for_placement called for {:?} at {:?}",
            self.block.key,
            context.relative_pos
        );
        Some(self.get_connection_state(context.world, &context.relative_pos))
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        _world: &World,
        _pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        neighbor_state: BlockStateId,
    ) -> BlockStateId {
        // TODO: Waterlogged

        // Only update for horizontal directions
        if direction.is_horizontal() && let Some(direction_connection) = direction.to_connection_property() {
            let connects = self.connects_to(neighbor_state, direction);
            state.set_value(&direction_connection, connects)
        } else {
            state
        }
    }
}
