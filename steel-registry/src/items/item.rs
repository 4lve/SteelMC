use std::io::{self, Read};

use steel_utils::{
    BlockPos,
    math::Vector3,
    serial::ReadFrom,
    types::{InteractionHand, UpdateFlags},
};

use crate::{
    REGISTRY,
    blocks::{BlockRef, properties::Direction},
    compat_traits::{RegistryPlayer, RegistryWorld},
    item_stack::ItemStack,
};

pub use crate::blocks::behaviour::BlockPlaceContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionResult {
    Success,
    Fail,
    Pass,
    TryEmptyHandInteraction,
}

impl InteractionResult {
    /// Returns true if this result consumes the action (Success or Fail).
    /// Pass and TryEmptyHandInteraction do not consume the action.
    #[must_use]
    pub fn consumes_action(&self) -> bool {
        matches!(self, InteractionResult::Success | InteractionResult::Fail)
    }
}

#[derive(Debug, Clone)]
pub struct BlockHitResult {
    pub location: Vector3<f64>,
    pub direction: Direction,
    pub block_pos: BlockPos,
    pub miss: bool,
    pub inside: bool,
    pub world_border_hit: bool,
}

impl ReadFrom for BlockHitResult {
    fn read(data: &mut impl Read) -> io::Result<Self> {
        let block_pos = BlockPos::read(data)?;
        let direction = Direction::read(data)?;
        // Click coordinates are relative to the block position (0.0 to 1.0 range)
        let click_x = f32::read(data)?;
        let click_y = f32::read(data)?;
        let click_z = f32::read(data)?;
        let inside = bool::read(data)?;
        let world_border_hit = bool::read(data)?;

        // Convert to absolute world coordinates by adding block position
        // (matching Java's FriendlyByteBuf.readBlockHitResult)
        let location = Vector3::new(
            f64::from(block_pos.x()) + f64::from(click_x),
            f64::from(block_pos.y()) + f64::from(click_y),
            f64::from(block_pos.z()) + f64::from(click_z),
        );

        Ok(BlockHitResult {
            location,
            direction,
            block_pos,
            miss: false,
            inside,
            world_border_hit,
        })
    }
}

pub struct UseOnContext<'a> {
    pub player: &'a dyn RegistryPlayer,
    pub hand: InteractionHand,
    pub hit_result: BlockHitResult,
    pub world: &'a dyn RegistryWorld,
    pub item_stack: &'a mut ItemStack,
}

/// Trait defining item behavior (use, placement, etc.)
pub trait ItemBehavior: Send + Sync {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult;
}

/// Default item behavior - does nothing special
pub struct DefaultItemBehavior;

impl ItemBehavior for DefaultItemBehavior {
    fn use_on(&self, _context: &mut UseOnContext) -> InteractionResult {
        InteractionResult::Pass
    }
}

/// Behavior for items that place blocks
pub struct BlockItemBehavior {
    pub block: BlockRef,
}

impl ItemBehavior for BlockItemBehavior {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        let clicked_pos = context.hit_result.block_pos;
        let clicked_state = context.world.get_block_state(&clicked_pos);

        // Get the clicked block to check if it's replaceable
        let clicked_block = REGISTRY.blocks.by_state_id(clicked_state);
        let clicked_replaceable = clicked_block.map(|b| b.config.replaceable).unwrap_or(false);

        // Determine placement position: replace clicked block if replaceable,
        // otherwise place adjacent to the clicked face
        let (place_pos, replace_clicked) = if clicked_replaceable {
            (clicked_pos, true)
        } else {
            (context.hit_result.direction.relative(&clicked_pos), false)
        };

        // Check if placement position is within world bounds
        if !context.world.is_in_valid_bounds(&place_pos) {
            return InteractionResult::Fail;
        }

        // Check if the placement position already has a non-replaceable block
        let existing_state = context.world.get_block_state(&place_pos);
        let existing_block = REGISTRY.blocks.by_state_id(existing_state);
        let existing_replaceable = existing_block
            .map(|b| b.config.replaceable)
            .unwrap_or(false);

        if !existing_replaceable {
            return InteractionResult::Fail;
        }

        // Create placement context
        // TODO: Calculate horizontal_direction from player rotation
        let place_context = BlockPlaceContext {
            clicked_pos,
            clicked_face: context.hit_result.direction,
            click_location: context.hit_result.location,
            inside: context.hit_result.inside,
            relative_pos: place_pos,
            replace_clicked,
            horizontal_direction: Direction::North, // TODO: Calculate from player rotation
            rotation: 0.0,                          // TODO: Get from player
            world: context.world,
        };

        // Get block state for placement from the block behavior
        let behavior = REGISTRY.blocks.get_behavior(self.block);
        let Some(new_state) = behavior.get_state_for_placement(&place_context) else {
            return InteractionResult::Fail;
        };

        // Place the block
        if !context
            .world
            .set_block(place_pos, new_state, UpdateFlags::all())
        {
            return InteractionResult::Fail;
        }

        // Consume one item from the stack (like Java's itemStack.consume(1, player))
        context.item_stack.shrink(1);

        // TODO: Play place sound
        // TODO: Call behavior.on_place()

        InteractionResult::Success
    }
}

impl BlockItemBehavior {
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}
