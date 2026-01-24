//! Context types and results for block and item interactions.

use steel_registry::blocks::properties::Direction;
use steel_registry::item_stack::ItemStack;
use steel_utils::BlockPos;
use steel_utils::math::Vector3;
use steel_utils::types::InteractionHand;

// Re-export BlockHitResult from steel-registry since it's also used by steel-protocol
pub use steel_registry::items::item::BlockHitResult;

use crate::player::Player;
use crate::world::World;

/// Result of an interaction (item use, block use, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionResult {
    /// The interaction succeeded and consumed the action.
    Success,
    /// The interaction failed and consumed the action.
    Fail,
    /// The interaction did not apply; try the next handler.
    Pass,
    /// Try the empty-hand interaction on the block.
    TryEmptyHandInteraction,
}

impl InteractionResult {
    /// Returns true if this result consumes the action (Success or Fail).
    /// Pass and `TryEmptyHandInteraction` do not consume the action.
    #[must_use]
    pub fn consumes_action(&self) -> bool {
        matches!(self, InteractionResult::Success | InteractionResult::Fail)
    }
}

/// Context for placing a block.
pub struct BlockPlaceContext<'a> {
    /// The position that was clicked.
    pub clicked_pos: BlockPos,
    /// The face of the block that was clicked.
    pub clicked_face: Direction,
    /// The exact location where the click occurred.
    pub click_location: Vector3<f64>,
    /// Whether the click was inside the block.
    pub inside: bool,
    /// The position where the block will be placed.
    pub relative_pos: BlockPos,
    /// Whether the clicked block is being replaced.
    pub replace_clicked: bool,
    /// The horizontal direction the player is facing.
    pub horizontal_direction: Direction,
    /// The player's rotation (yaw).
    pub rotation: f32,
    /// The player's pitch (vertical look angle).
    pub pitch: f32,
    /// The world where the block is being placed.
    pub world: &'a World,
}

impl BlockPlaceContext<'_> {
    /// Returns the direction the player is looking at most directly.
    ///
    /// This considers both yaw and pitch to determine the nearest direction
    /// among all 6 directions (UP, DOWN, NORTH, SOUTH, EAST, WEST).
    ///
    /// Based on Java's `Direction.orderedByNearest(Entity)[0]`.
    #[must_use]
    pub fn get_nearest_looking_direction(&self) -> Direction {
        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = (-self.rotation).to_radians();

        let pitch_sin = pitch_rad.sin();
        let pitch_cos = pitch_rad.cos();
        let yaw_sin = yaw_rad.sin();
        let yaw_cos = yaw_rad.cos();

        let x_pos = yaw_sin > 0.0;
        let y_pos = pitch_sin < 0.0;
        let z_pos = yaw_cos > 0.0;

        let x_yaw = if x_pos { yaw_sin } else { -yaw_sin };
        let y_mag = if y_pos { -pitch_sin } else { pitch_sin };
        let z_yaw = if z_pos { yaw_cos } else { -yaw_cos };

        let x_mag = x_yaw * pitch_cos;
        let z_mag = z_yaw * pitch_cos;

        let axis_x = if x_pos {
            Direction::East
        } else {
            Direction::West
        };
        let axis_y = if y_pos {
            Direction::Up
        } else {
            Direction::Down
        };
        let axis_z = if z_pos {
            Direction::South
        } else {
            Direction::North
        };

        // Return the direction with the largest magnitude
        if x_yaw > z_yaw {
            if y_mag > x_mag { axis_y } else { axis_x }
        } else if y_mag > z_mag {
            axis_y
        } else {
            axis_z
        }
    }
}

/// Context for using an item on a block.
pub struct UseOnContext<'a> {
    /// The player using the item.
    pub player: &'a Player,
    /// Which hand the item is in.
    pub hand: InteractionHand,
    /// Information about where the block was hit.
    pub hit_result: BlockHitResult,
    /// The world where the interaction is happening.
    pub world: &'a World,
    /// The item stack being used (mutable for consumption).
    pub item_stack: &'a mut ItemStack,
}
