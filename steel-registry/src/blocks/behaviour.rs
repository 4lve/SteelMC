use steel_utils::{BlockPos, BlockStateId, math::Vector3, types::InteractionHand};

use crate::REGISTRY;
use crate::compat_traits::{RegistryPlayer, RegistryWorld};
use crate::item_stack::ItemStack;
use crate::items::item::{BlockHitResult, InteractionResult};

use crate::blocks::BlockRef;
use crate::blocks::properties::Direction;
pub use crate::blocks::properties::NoteBlockInstrument;

#[derive(Debug, Clone, Copy)]
pub enum PushReaction {
    Normal,
    Destroy,
    Block,
    Ignore,
    PushOnly,
}

#[derive(Debug)]
pub struct BlockConfig {
    pub has_collision: bool,
    pub can_occlude: bool,
    pub explosion_resistance: f32,
    pub is_randomly_ticking: bool,
    pub force_solid_off: bool,
    pub force_solid_on: bool,
    pub push_reaction: PushReaction,
    pub friction: f32,
    pub speed_factor: f32,
    pub jump_factor: f32,
    pub dynamic_shape: bool,
    pub destroy_time: f32,
    pub ignited_by_lava: bool,
    pub liquid: bool,
    pub is_air: bool,
    pub requires_correct_tool_for_drops: bool,
    pub instrument: NoteBlockInstrument,
    pub replaceable: bool,
}

impl BlockConfig {
    /// Starts building a new set of block properties.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            has_collision: true,
            can_occlude: true,
            explosion_resistance: 0.0,
            is_randomly_ticking: false,
            force_solid_off: false,
            force_solid_on: false,
            push_reaction: PushReaction::Normal,
            friction: 0.6,
            speed_factor: 1.0,
            jump_factor: 1.0,
            dynamic_shape: false,
            destroy_time: 0.0,
            ignited_by_lava: false,
            liquid: false,
            is_air: false,
            requires_correct_tool_for_drops: false,
            instrument: NoteBlockInstrument::Harp,
            replaceable: false,
        }
    }

    #[must_use]
    pub const fn has_collision(mut self, has_collision: bool) -> Self {
        self.has_collision = has_collision;
        self
    }

    #[must_use]
    pub const fn can_occlude(mut self, can_occlude: bool) -> Self {
        self.can_occlude = can_occlude;
        self
    }

    #[must_use]
    pub const fn explosion_resistance(mut self, resistance: f32) -> Self {
        self.explosion_resistance = resistance;
        self
    }

    #[must_use]
    pub const fn is_randomly_ticking(mut self, ticking: bool) -> Self {
        self.is_randomly_ticking = ticking;
        self
    }

    #[must_use]
    pub const fn force_solid_off(mut self, force: bool) -> Self {
        self.force_solid_off = force;
        self
    }

    #[must_use]
    pub const fn force_solid_on(mut self, force: bool) -> Self {
        self.force_solid_on = force;
        self
    }

    #[must_use]
    pub const fn push_reaction(mut self, reaction: PushReaction) -> Self {
        self.push_reaction = reaction;
        self
    }

    #[must_use]
    pub const fn friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    #[must_use]
    pub const fn speed_factor(mut self, factor: f32) -> Self {
        self.speed_factor = factor;
        self
    }

    #[must_use]
    pub const fn jump_factor(mut self, factor: f32) -> Self {
        self.jump_factor = factor;
        self
    }

    #[must_use]
    pub const fn dynamic_shape(mut self, dynamic: bool) -> Self {
        self.dynamic_shape = dynamic;
        self
    }

    #[must_use]
    pub const fn destroy_time(mut self, time: f32) -> Self {
        self.destroy_time = time;
        self
    }

    #[must_use]
    pub const fn ignited_by_lava(mut self, ignited: bool) -> Self {
        self.ignited_by_lava = ignited;
        self
    }

    #[must_use]
    pub const fn liquid(mut self, liquid: bool) -> Self {
        self.liquid = liquid;
        self
    }

    #[must_use]
    pub const fn is_air(mut self, is_air: bool) -> Self {
        self.is_air = is_air;
        self
    }

    #[must_use]
    pub const fn requires_correct_tool_for_drops(mut self, requires: bool) -> Self {
        self.requires_correct_tool_for_drops = requires;
        self
    }

    #[must_use]
    pub const fn instrument(mut self, instrument: NoteBlockInstrument) -> Self {
        self.instrument = instrument;
        self
    }

    #[must_use]
    pub const fn replaceable(mut self, replaceable: bool) -> Self {
        self.replaceable = replaceable;
        self
    }
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BlockPlaceContext<'a> {
    pub clicked_pos: BlockPos,
    pub clicked_face: Direction,
    pub click_location: Vector3<f64>,
    pub inside: bool,
    pub relative_pos: BlockPos,
    pub replace_clicked: bool,
    pub horizontal_direction: Direction,
    pub rotation: f32,
    pub world: &'a dyn RegistryWorld,
}

pub trait BlockBehaviour: Send + Sync {
    /// Called when a neighboring block changes shape.
    /// Returns the new state for this block after considering the neighbor change.
    fn update_shape(
        &self,
        state: BlockStateId,
        _world: &dyn RegistryWorld,
        _pos: BlockPos,
        _direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        state
    }

    /// Returns the block state to use when placing this block.
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId>;

    /// Called when this block is placed in the world.
    ///
    /// # Arguments
    /// * `state` - The new block state that was placed
    /// * `world` - The world the block was placed in
    /// * `pos` - The position where the block was placed
    /// * `old_state` - The previous block state at this position
    /// * `moved_by_piston` - Whether the block was moved by a piston
    #[allow(unused_variables)]
    fn on_place(
        &self,
        state: BlockStateId,
        world: &dyn RegistryWorld,
        pos: BlockPos,
        old_state: BlockStateId,
        moved_by_piston: bool,
    ) {
        // Default: no-op
    }

    /// Called after this block is removed from the world, to affect neighbors.
    ///
    /// This is used for things like rails notifying neighbors when removed.
    ///
    /// # Arguments
    /// * `state` - The block state that was removed
    /// * `world` - The world the block was removed from
    /// * `pos` - The position where the block was removed
    /// * `moved_by_piston` - Whether the block was moved by a piston
    #[allow(unused_variables)]
    fn affect_neighbors_after_removal(
        &self,
        state: BlockStateId,
        world: &dyn RegistryWorld,
        pos: BlockPos,
        moved_by_piston: bool,
    ) {
        // Default: no-op
    }

    /// Called when a player uses an item on this block.
    ///
    /// Returns `TryEmptyHandInteraction` by default to fall through to item use.
    /// Override this to handle block-specific interactions (e.g., opening chests,
    /// using buttons, etc.).
    #[allow(unused_variables, clippy::too_many_arguments)]
    fn use_item_on(
        &self,
        item_stack: &ItemStack,
        state: BlockStateId,
        world: &dyn RegistryWorld,
        pos: BlockPos,
        player: &dyn RegistryPlayer,
        hand: InteractionHand,
        hit_result: &BlockHitResult,
    ) -> InteractionResult {
        InteractionResult::TryEmptyHandInteraction
    }

    /// Called when a player uses this block without an item (or as a fallback
    /// when `use_item_on` returns `TryEmptyHandInteraction`).
    ///
    /// Returns `Pass` by default. Override this for blocks that have interactions
    /// without needing an item (e.g., buttons, levers, repeaters).
    #[allow(unused_variables)]
    fn use_without_item(
        &self,
        state: BlockStateId,
        world: &dyn RegistryWorld,
        pos: BlockPos,
        player: &dyn RegistryPlayer,
        hit_result: &BlockHitResult,
    ) -> InteractionResult {
        InteractionResult::Pass
    }
}

/// A placeholder behavior that returns None for placement.
/// Used as an initial placeholder when blocks are registered, before their
/// actual behavior is set.
pub struct PlaceholderBlockBehaviour;

impl BlockBehaviour for PlaceholderBlockBehaviour {
    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        None
    }
}

/// Static instance of placeholder behavior for use during block registration.
pub static PLACEHOLDER_BEHAVIOR: PlaceholderBlockBehaviour = PlaceholderBlockBehaviour;

pub struct DefaultBlockBehaviour {
    block: BlockRef,
}

impl DefaultBlockBehaviour {
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehaviour for DefaultBlockBehaviour {
    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        let test = self.block.default_state();
        log::info!(
            "{:?}, {:?}, {:?}, {:?}",
            REGISTRY.blocks.get_properties(test),
            test,
            self.block.key,
            REGISTRY.blocks.by_state_id(test)
        );
        Some(self.block.default_state())
    }
}
