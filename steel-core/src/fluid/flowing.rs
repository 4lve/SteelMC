//! Core fluid flowing behavior.
//!
//! Based on vanilla's FlowingFluid.java.

use std::ptr;

use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{BlockStateProperties, Direction};
use steel_registry::vanilla_blocks;
use steel_registry::REGISTRY;
use steel_utils::BlockPos;
use steel_utils::BlockStateId;

use crate::world::World;

/// Type of fluid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FluidType {
    Empty,
    Water,
    Lava,
}

impl FluidType {
    /// Returns the tick delay for this fluid type.
    #[must_use]
    pub fn tick_delay(&self) -> u32 {
        match self {
            FluidType::Water => 5,
            FluidType::Lava => 30, // 10 in nether
            FluidType::Empty => 0,
        }
    }

    /// Returns the drop-off per block (level reduction).
    #[must_use]
    pub fn drop_off(&self) -> u8 {
        match self {
            FluidType::Water => 1,
            FluidType::Lava => 2, // 1 in nether
            FluidType::Empty => 0,
        }
    }

    /// Returns how far to search for drops (slope finding distance).
    #[must_use]
    pub fn slope_find_distance(&self) -> u8 {
        match self {
            FluidType::Water => 4,
            FluidType::Lava => 2, // 4 in nether
            FluidType::Empty => 0,
        }
    }
}

/// Represents a fluid state at a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidState {
    /// Type of fluid.
    pub fluid_type: FluidType,
    /// Level 0-8. 0 = source, 1-7 = flowing (higher = less water), 8 = falling.
    pub level: u8,
    /// Whether the fluid is falling (from above).
    pub falling: bool,
}

impl FluidState {
    /// Creates an empty fluid state.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            fluid_type: FluidType::Empty,
            level: 0,
            falling: false,
        }
    }

    /// Creates a source block fluid state.
    #[must_use]
    pub const fn source(fluid_type: FluidType) -> Self {
        Self {
            fluid_type,
            level: 0,
            falling: false,
        }
    }

    /// Creates a flowing fluid state.
    #[must_use]
    pub fn flowing(fluid_type: FluidType, level: u8, falling: bool) -> Self {
        Self {
            fluid_type,
            level: if level > 8 { 8 } else { level },
            falling,
        }
    }

    /// Returns true if this is a source block (level 0).
    #[must_use]
    pub fn is_source(&self) -> bool {
        self.level == 0 && !self.falling
    }

    /// Returns true if this is an empty fluid state.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fluid_type == FluidType::Empty
    }

    /// Returns the "amount" (inverse of level for compatibility).
    /// Source = 8, level 1 = 7, level 7 = 1.
    #[must_use]
    pub fn amount(&self) -> u8 {
        if self.is_source() {
            8
        } else if self.falling {
            8
        } else {
            8u8.saturating_sub(self.level)
        }
    }
}

/// Trait for fluid behavior implementations.
pub trait FluidBehaviour: Send + Sync {
    /// Returns the fluid type.
    fn fluid_type(&self) -> FluidType;

    /// Returns the tick delay for this fluid.
    fn tick_delay(&self) -> u32 {
        self.fluid_type().tick_delay()
    }

    /// Returns the drop-off per block.
    fn drop_off(&self) -> u8 {
        self.fluid_type().drop_off()
    }

    /// Called when a scheduled tick fires for this fluid.
    fn tick(&self, world: &World, pos: BlockPos, current_tick: u64);

    /// Spreads the fluid from the given position.
    fn spread(&self, world: &World, pos: BlockPos, fluid_state: FluidState, current_tick: u64);
}

/// Gets the fluid state at a block position.
///
/// This derives FluidState from BlockState (Option A approach for simplicity).
#[must_use]
pub fn get_fluid_state(world: &World, pos: &BlockPos) -> FluidState {
    let state = world.get_block_state(pos);
    get_fluid_state_from_block(state)
}

/// Gets the fluid state from a block state.
#[must_use]
pub fn get_fluid_state_from_block(state: BlockStateId) -> FluidState {
    let block = state.get_block();

    if ptr::eq(block, vanilla_blocks::WATER) {
        let level: u8 = state.try_get_value(&BlockStateProperties::LEVEL).unwrap_or(0);
        if level == 0 {
            FluidState::source(FluidType::Water)
        } else {
            // Level 8+ means falling
            FluidState::flowing(FluidType::Water, level.min(7), level >= 8)
        }
    } else if ptr::eq(block, vanilla_blocks::LAVA) {
        let level: u8 = state.try_get_value(&BlockStateProperties::LEVEL).unwrap_or(0);
        if level == 0 {
            FluidState::source(FluidType::Lava)
        } else {
            FluidState::flowing(FluidType::Lava, level.min(7), level >= 8)
        }
    } else {
        // Check waterlogged property
        if let Some(true) = state.try_get_value(&BlockStateProperties::WATERLOGGED) {
            FluidState::source(FluidType::Water)
        } else {
            FluidState::empty()
        }
    }
}

/// Checks if a block can be replaced by fluid.
#[must_use]
pub fn can_be_replaced_by_fluid(world: &World, pos: &BlockPos) -> bool {
    let state = world.get_block_state(pos);
    let block = state.get_block();

    // Air and replaceable blocks can be replaced
    block.config.replaceable || block.config.is_air
}

/// Checks if fluid can pass through a wall between two positions.
#[must_use]
pub fn can_pass_through(world: &World, _from: BlockPos, to: BlockPos, _direction: Direction) -> bool {
    // Simplified check: just verify the target is in bounds and replaceable
    if !world.is_in_valid_bounds(&to) {
        return false;
    }

    let to_state = world.get_block_state(&to);
    let to_block = to_state.get_block();

    // Can't pass through solid blocks
    if !to_block.config.replaceable && !to_block.config.is_air {
        // Check if it's already the same fluid
        let fluid = get_fluid_state_from_block(to_state);
        if fluid.is_empty() {
            return false;
        }
    }

    true
}

/// Calculates the new fluid state for a position based on neighbors.
/// This is vanilla's getNewLiquid() function.
///
/// Returns the fluid state that should exist at this position.
/// A fluid block can only be supported by:
/// - A source block nearby
/// - Water directly above (creates falling water with level 8)  
/// - A neighbor with HIGHER amount (lower level) that can flow into this position
#[must_use]
pub fn get_new_liquid(world: &World, pos: BlockPos, fluid_type: FluidType) -> FluidState {
    let drop_off = fluid_type.drop_off();
    let mut max_incoming_amount = 0u8;
    let mut source_count = 0u8;

    // Check horizontal neighbors for water that could flow INTO this position
    for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
        let neighbor_pos = direction.relative(&pos);
        let neighbor_fluid = get_fluid_state(world, &neighbor_pos);

        if neighbor_fluid.fluid_type == fluid_type {
            if neighbor_fluid.is_source() {
                source_count += 1;
                // Source can provide amount 8, minus drop_off
                let incoming = 8u8.saturating_sub(drop_off);
                max_incoming_amount = max_incoming_amount.max(incoming);
            } else {
                // Flowing water (including falling): calculate what amount it would provide
                // Falling water has amount=8, so it provides strong horizontal support
                let incoming = neighbor_fluid.amount().saturating_sub(drop_off);
                max_incoming_amount = max_incoming_amount.max(incoming);
            }
        }
    }

    // Check above for falling fluid - vanilla uses getFlowing(8, true)
    let above_pos = pos.offset(0, 1, 0);
    let above_fluid = get_fluid_state(world, &above_pos);
    if above_fluid.fluid_type == fluid_type {
        // Water above should create falling water here (level 8, falling=true)
        return FluidState::flowing(fluid_type, 8, true);
    }

    // Water source conversion: 2+ adjacent sources + solid below = new source
    if fluid_type == FluidType::Water && source_count >= 2 {
        let below_pos = pos.offset(0, -1, 0);
        let below_state = world.get_block_state(&below_pos);
        let below_block = below_state.get_block();
        let below_fluid = get_fluid_state_from_block(below_state);
        // Solid block OR source of same type below
        if (!below_block.config.replaceable && !below_block.config.is_air) 
            || below_fluid.is_source() {
            return FluidState::source(FluidType::Water);
        }
    }

    // If we have incoming flow, calculate new state
    if max_incoming_amount > 0 {
        let new_level = 8 - max_incoming_amount;
        FluidState::flowing(fluid_type, new_level, false)
    } else {
        // No support = empty
        FluidState::empty()
    }
}

/// Alias for backwards compatibility
#[must_use]
pub fn calculate_new_fluid(world: &World, pos: BlockPos, fluid_type: FluidType) -> FluidState {
    get_new_liquid(world, pos, fluid_type)
}

/// Converts a FluidState to a BlockStateId for the corresponding fluid block.
/// 
/// Block state LEVEL property:
/// - 0 = source
/// - 1-7 = flowing water (1 = most, 7 = least)
/// - 8 = falling water (from above)
#[must_use]
pub fn fluid_state_to_block(fluid_state: FluidState) -> BlockStateId {
    match fluid_state.fluid_type {
        FluidType::Empty => REGISTRY.blocks.get_default_state_id(vanilla_blocks::AIR),
        FluidType::Water => {
            let base = REGISTRY.blocks.get_default_state_id(vanilla_blocks::WATER);
            // Falling water uses LEVEL=8, non-falling uses the actual level
            let level = if fluid_state.falling { 8 } else { fluid_state.level };
            base.set_value(&BlockStateProperties::LEVEL, level)
        }
        FluidType::Lava => {
            let base = REGISTRY.blocks.get_default_state_id(vanilla_blocks::LAVA);
            let level = if fluid_state.falling { 8 } else { fluid_state.level };
            base.set_value(&BlockStateProperties::LEVEL, level)
        }
    }
}

// ============================================================================
// SLOPE FINDING ALGORITHM
// ============================================================================

/// Checks if the given position is a "hole" where water can fall down.
/// A hole is when the block below is air/replaceable or is the same fluid type.
#[must_use]
pub fn is_hole(world: &World, pos: &BlockPos, fluid_type: FluidType) -> bool {
    let below = pos.offset(0, -1, 0);
    
    if !world.is_in_valid_bounds(&below) {
        return false;
    }
    
    let below_state = world.get_block_state(&below);
    let below_block = below_state.get_block();
    
    // Check if we can flow down
    if below_block.config.is_air || below_block.config.replaceable {
        return true;
    }
    
    // Check if below is same fluid (water can flow into water)
    let below_fluid = get_fluid_state_from_block(below_state);
    if below_fluid.fluid_type == fluid_type && !below_fluid.is_source() {
        return true;
    }
    
    false
}

/// Checks if water can pass through to a position horizontally.
#[must_use]
fn can_pass_horizontally(world: &World, pos: &BlockPos, fluid_type: FluidType) -> bool {
    if !world.is_in_valid_bounds(pos) {
        return false;
    }
    
    let state = world.get_block_state(pos);
    let block = state.get_block();
    
    // Can flow into air or replaceable blocks
    if block.config.is_air || block.config.replaceable {
        return true;
    }
    
    // Can flow into same fluid type if not source
    let fluid = get_fluid_state_from_block(state);
    if fluid.fluid_type == fluid_type && !fluid.is_source() {
        return true;
    }
    
    false
}

/// Recursively calculates the distance to the nearest hole in a given direction.
///
/// Returns the distance (depth) at which a hole was found, or 1000 if no hole found.
/// 
/// # Arguments
/// * `world` - The world to search in
/// * `pos` - Current position to check from
/// * `depth` - Current search depth
/// * `from_direction` - Direction we came from (to avoid going back)
/// * `fluid_type` - Type of fluid we're searching for
/// * `max_depth` - Maximum search depth (slope_find_distance)
fn get_slope_distance(
    world: &World,
    pos: BlockPos,
    depth: u8,
    from_direction: Option<Direction>,
    fluid_type: FluidType,
    max_depth: u8,
) -> u16 {
    let mut min_distance: u16 = 1000;
    
    // Check all horizontal directions except the one we came from
    for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
        // Skip the direction we came from
        if let Some(from) = from_direction {
            if direction == from.opposite() {
                continue;
            }
        }
        
        let neighbor = direction.relative(&pos);
        
        // Can we pass through to this neighbor?
        if !can_pass_horizontally(world, &neighbor, fluid_type) {
            continue;
        }
        
        // Is this position a hole?
        if is_hole(world, &neighbor, fluid_type) {
            return depth as u16; // Found a hole at this depth
        }
        
        // If we haven't reached max depth, continue searching
        if depth < max_depth {
            let distance = get_slope_distance(
                world,
                neighbor,
                depth + 1,
                Some(direction),
                fluid_type,
                max_depth,
            );
            if distance < min_distance {
                min_distance = distance;
            }
        }
    }
    
    min_distance
}

/// Gets the spread map for water, like vanilla's getSpread().
///
/// Returns a list of (Direction, FluidState) pairs to spread to.
/// Uses slope finding to prioritize directions toward holes.
/// For each direction, calculates the correct FluidState using get_new_liquid.
#[must_use]
pub fn get_spread(world: &World, pos: BlockPos, fluid_type: FluidType) -> Vec<(Direction, FluidState)> {
    let max_depth = fluid_type.slope_find_distance();
    let mut candidates: Vec<(Direction, FluidState, u16)> = Vec::new();
    
    for direction in [Direction::North, Direction::South, Direction::East, Direction::West] {
        let neighbor = direction.relative(&pos);
        
        // Can we flow there?
        if !can_pass_horizontally(world, &neighbor, fluid_type) {
            continue;
        }
        
        // Calculate what fluid should exist at the neighbor position
        // This is the key insight from vanilla - each position calculates its own state
        let new_fluid = get_new_liquid(world, neighbor, fluid_type);
        
        // Skip if no valid fluid would be placed
        if new_fluid.is_empty() {
            continue;
        }
        
        // Calculate slope distance
        let distance = if is_hole(world, &neighbor, fluid_type) {
            0
        } else if max_depth > 0 {
            get_slope_distance(world, neighbor, 1, Some(direction), fluid_type, max_depth)
        } else {
            1000
        };
        
        candidates.push((direction, new_fluid, distance));
    }
    
    if candidates.is_empty() {
        return Vec::new();
    }
    
    // Find the minimum distance
    let min_distance = candidates.iter().map(|(_, _, d)| *d).min().unwrap_or(1000);
    
    // Only return directions with the minimum distance (ties are allowed)
    candidates
        .into_iter()
        .filter(|(_, _, d)| *d == min_distance)
        .map(|(dir, fluid, _)| (dir, fluid))
        .collect()
}

/// Backwards compatible wrapper - returns just directions
#[must_use]
pub fn get_spread_directions(world: &World, pos: BlockPos, fluid_type: FluidType) -> Vec<Direction> {
    get_spread(world, pos, fluid_type)
        .into_iter()
        .map(|(dir, _)| dir)
        .collect()
}
