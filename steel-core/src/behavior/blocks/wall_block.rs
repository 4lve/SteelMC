//! bar block behavior implementation.
//!
//! bars connect to adjacent bars, bar solid blocks.

use steel_registry::REGISTRY;
use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, EnumProperty, WallSide,
};
use steel_utils::{BlockPos, BlockStateId, Identifier};

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::BlockPlaceContext;
use crate::world::World;

/// Behavior for bar blocks.
///
/// bars have 4 boolean properties (north, east, south, west) that indicate
/// whether the bar connects in that direction. A bar connects to:
/// - Other bars of the same type
/// - bar gates facing the appropriate direction
/// - Blocks with a sturdy face on the connecting side
pub struct WallBlock {
    block: BlockRef,
}

impl WallBlock {
    /// North connection property.
    pub const NORTH: EnumProperty<WallSide> = BlockStateProperties::NORTH_WALL;
    /// East connection property.
    pub const EAST: EnumProperty<WallSide> = BlockStateProperties::EAST_WALL;
    /// South connection property.
    pub const SOUTH: EnumProperty<WallSide> = BlockStateProperties::SOUTH_WALL;
    /// West connection property.
    pub const WEST: EnumProperty<WallSide> = BlockStateProperties::WEST_WALL;
    /// Waterlogged property.
    pub const WATERLOGGED: BoolProperty = BlockStateProperties::WATERLOGGED;

    /// Creates a new bar block behavior for the given block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Checks if this bar should connect to the given neighbor state.
    fn connects_with(neighbor_state: BlockStateId, direction: Direction) -> WallSide {
        let neighbor_block = neighbor_state.get_block();
        // Check if it's a bar (same tag)
        let bars_tag = Identifier::vanilla_static("bars");
        if REGISTRY.blocks.is_in_tag(neighbor_block, &bars_tag) {
            return WallSide::Low;
        }
        let walls_tag = Identifier::vanilla_static("walls");
        if REGISTRY.blocks.is_in_tag(neighbor_block, &walls_tag) {
            return WallSide::Low;
        }
        if direction == Direction::Up{
            let walls_tag = Identifier::vanilla_static("fence");
            if REGISTRY.blocks.is_in_tag(neighbor_block, &walls_tag) {
                return WallSide::Tall;
            }
        }
        // TODO glass is not in minecraft: it is in c: so this needed to be fixed and worked on
        let glass_tag = Identifier::vanilla_static("glass_pane");
        if REGISTRY.blocks.is_in_tag(neighbor_block, &glass_tag) {
            return WallSide::Low;
        }

        // Check if the neighbor has a sturdy face on the opposite side
        let opposite = match direction {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        };
        if neighbor_state.is_face_sturdy(opposite)
        {
            return WallSide::Low
        }
        WallSide::None
    }

    /// Gets the connection state for a position by checking all 4 horizontal neighbors.
    fn get_connection_state(&self, world: &World, pos: &BlockPos) -> BlockStateId {
        let mut state = self.block.default_state();

        // Check north
        let north_pos = Direction::North.relative(pos);
        let north_state = world.get_block_state(&north_pos);
        let connects_north = Self::connects_with(north_state, Direction::North);
        state = state.set_value(&Self::NORTH, connects_north);

        // Check east
        let east_pos = Direction::East.relative(pos);
        let east_state = world.get_block_state(&east_pos);
        let connects_east = Self::connects_with(east_state, Direction::East);
        state = state.set_value(&Self::EAST, connects_east);

        // Check south
        let south_pos = Direction::South.relative(pos);
        let south_state = world.get_block_state(&south_pos);
        let connects_south = Self::connects_with(south_state, Direction::South);
        state = state.set_value(&Self::SOUTH, connects_south);

        // Check west
        let west_pos = Direction::West.relative(pos);
        let west_state = world.get_block_state(&west_pos);
        let connects_west = Self::connects_with(west_state, Direction::West);
        state = state.set_value(&Self::WEST, connects_west);

        // Check Up
        let up_pos = Direction::Up.relative(pos);
        let up_state = world.get_block_state(&up_pos);
        let connects_west = Self::connects_with(up_state, Direction::Up);
        state = state.set_value(&Self::WEST, connects_west);

        state
    }
}

impl BlockBehaviour for WallBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
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
        // Only update for horizontal directions
        match direction {
            Direction::North => {
                let connects = Self::connects_with(neighbor_state, Direction::North);
                state.set_value(&Self::NORTH, connects)
            }
            Direction::East => {
                let connects = Self::connects_with(neighbor_state, Direction::East);
                state.set_value(&Self::EAST, connects)
            }
            Direction::South => {
                let connects = Self::connects_with(neighbor_state, Direction::South);
                state.set_value(&Self::SOUTH, connects)
            }
            Direction::West => {
                let connects = Self::connects_with(neighbor_state, Direction::West);
                state.set_value(&Self::WEST, connects)
            }
            // Vertical directions don't affect bar connections
            Direction::Up | Direction::Down => state,
        }
    }
}
