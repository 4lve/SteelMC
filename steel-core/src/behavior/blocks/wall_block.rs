//! Wall block behavior implementation.
//!
//! Walls connect to adjacent walls, solid blocks, fence gates, iron bars, and glass panes.
//! Walls have a center post (controlled by the UP property) that renders based on
//! connection patterns and the block above.

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

/// Behavior for wall blocks.
///
/// Walls have 4 enum properties (north, east, south, west) with values `None`, `Low`, or `Tall`
/// that indicate how the wall connects in that direction, plus an `UP` boolean property
/// that controls whether the center post is visible. A wall connects to:
/// - Other wall blocks
/// - Blocks with a sturdy face on the connecting side
/// - Iron bars and copper bars
/// - Glass panes
/// - Fence gates facing perpendicular to the wall direction
pub struct WallBlock {
    block: BlockRef,
}

impl WallBlock {
    /// Up property - controls center post visibility.
    pub const UP: BoolProperty = BlockStateProperties::UP;
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

    /// Creates a new wall block behavior for the given block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Checks if this wall should connect to the given neighbor state in the given direction.
    ///
    /// A wall connects to:
    /// - Other walls (`minecraft:walls` tag)
    /// - Iron bars and copper bars (`minecraft:bars` tag)
    /// - Glass panes (blocks with `glass_pane` in their name)
    /// - Fence gates facing perpendicular to the connection direction
    /// - Blocks with a sturdy face on the opposite side
    fn connects_to(neighbor_state: BlockStateId, direction: Direction) -> bool {
        let neighbor_block = neighbor_state.get_block();

        // Check if it's a wall
        let walls_tag = Identifier::vanilla_static("walls");
        if REGISTRY.blocks.is_in_tag(neighbor_block, &walls_tag) {
            return true;
        }

        // Check if it's iron bars or copper bars
        let bars_tag = Identifier::vanilla_static("bars");
        if REGISTRY.blocks.is_in_tag(neighbor_block, &bars_tag) {
            return true;
        }

        // Check if it's a glass pane (no c:glass_panes tag available, check by name)
        if neighbor_block.key.path.contains("glass_pane") {
            return true;
        }

        // Check if it's a fence gate facing perpendicular to the connection direction
        let fence_gates_tag = Identifier::vanilla_static("fence_gates");
        if REGISTRY.blocks.is_in_tag(neighbor_block, &fence_gates_tag)
            && let Some(facing_str) = neighbor_state.get_property_str("facing")
        {
            let gate_facing = match facing_str.as_str() {
                "north" => Some(Direction::North),
                "south" => Some(Direction::South),
                "east" => Some(Direction::East),
                "west" => Some(Direction::West),
                _ => None,
            };

            if let Some(gate_facing) = gate_facing {
                // Gate connects perpendicular to its facing direction
                // A gate facing N/S connects to walls checking from E/W
                // A gate facing E/W connects to walls checking from N/S
                let connects = matches!(
                    (gate_facing, direction),
                    (
                        Direction::North | Direction::South,
                        Direction::East | Direction::West
                    ) | (
                        Direction::East | Direction::West,
                        Direction::North | Direction::South
                    )
                );
                if connects {
                    return true;
                }
            }
        }

        // Check if the neighbor has a sturdy face on the opposite side
        neighbor_state.is_face_sturdy(direction.opposite())
    }

    /// Determines the wall height (Tall/Low/None) for a connected side based on the block above.
    ///
    /// - Returns `None` if the wall doesn't connect in this direction
    /// - Returns `Tall` if the block above forces a tall connection:
    ///   - Full cube block above
    ///   - Wall above with connection in same direction
    ///   - Iron bars/glass pane above with connection in same direction
    ///   - Closed fence gate above perpendicular to this direction
    /// - Returns `Low` otherwise
    fn get_wall_height(
        connected: bool,
        above_state: BlockStateId,
        direction: Direction,
    ) -> WallSide {
        if !connected {
            return WallSide::None;
        }

        let above_block = above_state.get_block();

        // Check if block above is a full cube (has sturdy face on bottom)
        if above_state.is_face_sturdy(Direction::Down) {
            return WallSide::Tall;
        }

        // Check if block above is a wall with connection in the same direction
        let walls_tag = Identifier::vanilla_static("walls");
        if REGISTRY.blocks.is_in_tag(above_block, &walls_tag) {
            let wall_side = Self::get_side_property_value(above_state, direction);
            if wall_side != WallSide::None {
                return WallSide::Tall;
            }
        }

        // Check if block above is iron bars with connection in the same direction
        let bars_tag = Identifier::vanilla_static("bars");
        if REGISTRY.blocks.is_in_tag(above_block, &bars_tag)
            && Self::has_bool_connection(above_state, direction)
        {
            return WallSide::Tall;
        }

        // Check if block above is a glass pane with connection in the same direction
        if above_block.key.path.contains("glass_pane")
            && Self::has_bool_connection(above_state, direction)
        {
            return WallSide::Tall;
        }

        // Check if block above is a fence with connection in the same direction
        let fences_tag = Identifier::vanilla_static("fences");
        if REGISTRY.blocks.is_in_tag(above_block, &fences_tag)
            && Self::has_bool_connection(above_state, direction)
        {
            return WallSide::Tall;
        }

        // Check if block above is a closed fence gate perpendicular to this direction
        let fence_gates_tag = Identifier::vanilla_static("fence_gates");
        if REGISTRY.blocks.is_in_tag(above_block, &fence_gates_tag)
            && above_state
                .get_property_str("open")
                .is_some_and(|s| s == "false")
            && let Some(facing_str) = above_state.get_property_str("facing")
        {
            let gate_facing = match facing_str.as_str() {
                "north" => Some(Direction::North),
                "south" => Some(Direction::South),
                "east" => Some(Direction::East),
                "west" => Some(Direction::West),
                _ => None,
            };

            if let Some(gate_facing) = gate_facing {
                // Closed gate perpendicular to this direction forces Tall
                let perpendicular = matches!(
                    (gate_facing, direction),
                    (
                        Direction::North | Direction::South,
                        Direction::East | Direction::West
                    ) | (
                        Direction::East | Direction::West,
                        Direction::North | Direction::South
                    )
                );
                if perpendicular {
                    return WallSide::Tall;
                }
            }
        }

        WallSide::Low
    }

    /// Gets the `WallSide` property value for a given direction from a wall block state.
    fn get_side_property_value(state: BlockStateId, direction: Direction) -> WallSide {
        match direction {
            Direction::North => state.try_get_value(&Self::NORTH).unwrap_or(WallSide::None),
            Direction::East => state.try_get_value(&Self::EAST).unwrap_or(WallSide::None),
            Direction::South => state.try_get_value(&Self::SOUTH).unwrap_or(WallSide::None),
            Direction::West => state.try_get_value(&Self::WEST).unwrap_or(WallSide::None),
            Direction::Up | Direction::Down => WallSide::None,
        }
    }

    /// Checks if a block state has a boolean connection property set to true for the given direction.
    /// Used for iron bars, glass panes, and fences which use boolean properties.
    fn has_bool_connection(state: BlockStateId, direction: Direction) -> bool {
        let prop_name = match direction {
            Direction::North => "north",
            Direction::East => "east",
            Direction::South => "south",
            Direction::West => "west",
            Direction::Up | Direction::Down => return false,
        };

        state
            .get_property_str(prop_name)
            .is_some_and(|v| v == "true")
    }

    /// Determines if the center post should be raised (UP property).
    ///
    /// The post is raised if:
    /// - The wall has corners (not a straight line or cross pattern)
    /// - The wall is isolated (no connections)
    /// - The block above is a wall with UP=true
    /// - The block above is a closed fence gate parallel to the wall line
    fn should_raise_post(
        north: &WallSide,
        east: &WallSide,
        south: &WallSide,
        west: &WallSide,
        above_state: BlockStateId,
    ) -> bool {
        let has_north = *north != WallSide::None;
        let has_east = *east != WallSide::None;
        let has_south = *south != WallSide::None;
        let has_west = *west != WallSide::None;

        // Check if this is a straight line or cross pattern
        let connected_north_south = has_north && has_south && !has_east && !has_west;
        let connected_east_west = has_east && has_west && !has_north && !has_south;
        let is_cross = has_north && has_south && has_east && has_west;

        // If wall has corners or is isolated, always show post
        let is_straight_or_cross = is_cross || connected_north_south || connected_east_west;
        if !is_straight_or_cross {
            return true;
        }

        // For straight walls and crosses, check block above
        let above_block = above_state.get_block();

        // Check if block above is a wall - copy its UP value
        let walls_tag = Identifier::vanilla_static("walls");
        if REGISTRY.blocks.is_in_tag(above_block, &walls_tag) {
            return above_state.try_get_value(&Self::UP).unwrap_or(true);
        }
        // Check if block above is a fence - show post unless there's a straight Tall line
        let fences_tag = Identifier::vanilla_static("fences");
        if REGISTRY.blocks.is_in_tag(above_block, &fences_tag) {
            // Show post if there's no straight line of Tall connections (0 or 1 Tall side)
            let tall_ns_line = *north == WallSide::Tall && *south == WallSide::Tall;
            let tall_ew_line = *east == WallSide::Tall && *west == WallSide::Tall;
            if !tall_ns_line && !tall_ew_line {
                return true;
            }
        }

        // Check if block above is a fence gate
        let fence_gates_tag = Identifier::vanilla_static("fence_gates");
        if REGISTRY.blocks.is_in_tag(above_block, &fence_gates_tag) {
            // Get gate state
            let is_open = above_state
                .get_property_str("open")
                .is_some_and(|v| v == "true");

            // Open fence gate: no post
            if is_open {
                return false;
            }

            // Closed fence gate: check if parallel to wall line
            if let Some(facing_str) = above_state.get_property_str("facing") {
                let gate_faces_ns = facing_str == "north" || facing_str == "south";
                let gate_faces_ew = facing_str == "east" || facing_str == "west";

                // Closed gate parallel to wall line: show post
                if (connected_north_south && gate_faces_ns)
                    || (connected_east_west && gate_faces_ew)
                {
                    return true;
                }
            }
        }
        if (connected_east_west && *east != *west)
            || (connected_north_south && *north != *south)
            || (is_cross && (*east != *west || *north != *south))
        {
            return true;
        }

        // Otherwise, straight wall with no special block above: no post
        false
    }

    /// Computes the full wall state from the world at the given position.
    fn compute_wall_state(&self, world: &World, pos: &BlockPos) -> BlockStateId {
        // Get neighbor states
        let north_pos = Direction::North.relative(pos);
        let east_pos = Direction::East.relative(pos);
        let south_pos = Direction::South.relative(pos);
        let west_pos = Direction::West.relative(pos);
        let above_pos = Direction::Up.relative(pos);

        let north_state = world.get_block_state(&north_pos);
        let east_state = world.get_block_state(&east_pos);
        let south_state = world.get_block_state(&south_pos);
        let west_state = world.get_block_state(&west_pos);
        let above_state = world.get_block_state(&above_pos);

        // Check connections
        let connects_north = Self::connects_to(north_state, Direction::North);
        let connects_east = Self::connects_to(east_state, Direction::East);
        let connects_south = Self::connects_to(south_state, Direction::South);
        let connects_west = Self::connects_to(west_state, Direction::West);

        // Determine wall heights based on block above
        let north = Self::get_wall_height(connects_north, above_state, Direction::North);
        let east = Self::get_wall_height(connects_east, above_state, Direction::East);
        let south = Self::get_wall_height(connects_south, above_state, Direction::South);
        let west = Self::get_wall_height(connects_west, above_state, Direction::West);

        // Determine if post should be raised
        let up = Self::should_raise_post(&north, &east, &south, &west, above_state);

        // Build the state
        self.block
            .default_state()
            .set_value(&Self::UP, up)
            .set_value(&Self::NORTH, north.clone())
            .set_value(&Self::EAST, east.clone())
            .set_value(&Self::SOUTH, south.clone())
            .set_value(&Self::WEST, west.clone())
    }
}

impl BlockBehaviour for WallBlock {
    fn update_shape(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        // Down direction doesn't affect wall state
        if direction == Direction::Down {
            return state;
        }

        // For any other direction (horizontal or up), recalculate everything
        // because wall height depends on both horizontal neighbors AND block above
        self.compute_wall_state(world, &pos)
    }

    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(
            self.compute_wall_state(context.world, &context.relative_pos)
                .set_value(&Self::WATERLOGGED, context.is_water_source()),
        )
    }
}
