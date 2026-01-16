//! Note block behavior implementation.
//!
//! Note blocks change their instrument based on the block above or below them.
//! When right-clicked, they cycle through 25 notes (0-24).

use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, IntProperty, NoteBlockInstrument,
};
use steel_utils::{BlockPos, BlockStateId, types::UpdateFlags};

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::player::Player;
use crate::world::World;

/// Behavior for note blocks.
///
/// Note blocks have three properties:
/// - `instrument` (NoteBlockInstrument): determined by block above or below
/// - `note` (0-24): the pitch, cycled on right-click
/// - `powered` (bool): whether receiving redstone signal
pub struct NoteBlock {
    block: BlockRef,
}

impl NoteBlock {
    /// The instrument property.
    pub const INSTRUMENT: steel_registry::blocks::properties::EnumProperty<NoteBlockInstrument> =
        BlockStateProperties::NOTEBLOCK_INSTRUMENT;
    /// The note property (0-24).
    pub const NOTE: IntProperty = BlockStateProperties::NOTE;
    /// The powered property.
    pub const POWERED: BoolProperty = BlockStateProperties::POWERED;

    /// Creates a new note block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Determines the instrument based on blocks above and below the note block.
    ///
    /// Logic from vanilla:
    /// 1. Check block above - if its instrument works from above, use it
    /// 2. Otherwise check block below - use its instrument (or harp if it works from above)
    fn determine_instrument(world: &World, pos: &BlockPos) -> NoteBlockInstrument {
        // Check block above
        let above_pos = Direction::Up.relative(pos);
        let above_state = world.get_block_state(&above_pos);
        let instrument_above = above_state.get_block().config.instrument.clone();

        if instrument_above.works_above_note_block() {
            return instrument_above;
        }

        // Check block below
        let below_pos = Direction::Down.relative(pos);
        let below_state = world.get_block_state(&below_pos);
        let instrument_below = below_state.get_block().config.instrument.clone();

        // If below block's instrument works from above (mob head below shouldn't affect note block),
        // fall back to harp
        if instrument_below.works_above_note_block() {
            NoteBlockInstrument::Harp
        } else {
            instrument_below
        }
    }

    /// Sets the instrument property based on surrounding blocks.
    fn set_instrument(world: &World, pos: &BlockPos, state: BlockStateId) -> BlockStateId {
        let instrument = Self::determine_instrument(world, pos);
        state.set_value(&Self::INSTRUMENT, instrument)
    }

    /// Plays the note block (sends block event and game event).
    /// Currently a stub - sound playing will be implemented later.
    #[allow(unused_variables)]
    fn play_note(&self, player: Option<&Player>, state: BlockStateId, world: &World, pos: &BlockPos) {
        let instrument: NoteBlockInstrument = state.get_value(&Self::INSTRUMENT);

        // Only play if instrument works from above OR block above is air
        if instrument.works_above_note_block() {
            // Mob head instruments always play
            log::debug!("Note block at {:?} would play {:?} sound", pos, instrument);
        } else {
            // Check if block above is air
            let above_pos = Direction::Up.relative(pos);
            let above_state = world.get_block_state(&above_pos);
            if above_state.is_air() {
                let note: u8 = state.get_value(&Self::NOTE);
                log::debug!(
                    "Note block at {:?} would play {:?} at note {}",
                    pos,
                    instrument,
                    note
                );
            }
        }
        // TODO: Actually play the sound via block event system
        // TODO: Emit game event for sculk sensors
    }
}

impl BlockBehaviour for NoteBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        let state = self.block.default_state();
        Some(Self::set_instrument(context.world, &context.relative_pos, state))
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        // Only update instrument when block above or below changes
        if direction == Direction::Up || direction == Direction::Down {
            Self::set_instrument(world, &pos, state)
        } else {
            state
        }
    }

    fn use_without_item(
        &self,
        state: BlockStateId,
        world: &World,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
    ) -> InteractionResult {
        // Cycle the note property (0-24)
        let current_note: u8 = state.get_value(&Self::NOTE);
        let next_note = if current_note >= Self::NOTE.max {
            Self::NOTE.min
        } else {
            current_note + 1
        };
        let new_state = state.set_value(&Self::NOTE, next_note);

        // Update the block in the world
        world.set_block(pos, new_state, UpdateFlags::UPDATE_ALL);

        // Play the note
        self.play_note(Some(player), new_state, world, &pos);

        // TODO: Award stat Stats::TUNE_NOTEBLOCK

        InteractionResult::Success
    }
}
