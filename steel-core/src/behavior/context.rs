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
    /// The world where the block is being placed.
    pub world: &'a World,
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
