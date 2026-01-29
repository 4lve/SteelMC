//! Flint and steel item behavior.

use steel_protocol::packets::game::SoundSource;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::Direction;
use steel_registry::sound_events;
use steel_registry::vanilla_blocks;
use steel_utils::BlockPos;
use steel_utils::types::UpdateFlags;

use crate::behavior::ItemBehavior;
use crate::behavior::context::{InteractionResult, UseOnContext};

/// Behavior for flint and steel.
///
/// When used on a block, places fire at the adjacent position if possible.
pub struct FlintAndSteelBehavior;

impl FlintAndSteelBehavior {
    /// Checks if fire can be placed at the given position.
    fn can_place_fire(context: &UseOnContext, pos: &BlockPos) -> bool {
        // Must be in valid bounds
        if !context.world.is_in_valid_bounds(pos) {
            return false;
        }

        // Must be air or replaceable
        let state = context.world.get_block_state(pos);
        if !state.get_block().config.is_air && !state.get_block().config.replaceable {
            return false;
        }

        // Must have a solid block below OR an adjacent flammable block
        let below = pos.offset(0, -1, 0);
        let below_state = context.world.get_block_state(&below);
        if below_state.is_face_sturdy(Direction::Up) {
            return true;
        }

        // Check for adjacent flammable blocks
        for direction in Direction::VALUES {
            let neighbor = direction.relative(pos);
            let neighbor_state = context.world.get_block_state(&neighbor);
            if neighbor_state.get_block().config.ignite_odds > 0 {
                return true;
            }
        }

        false
    }
}

impl ItemBehavior for FlintAndSteelBehavior {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        let clicked_pos = context.hit_result.block_pos;
        let fire_pos = context.hit_result.direction.relative(&clicked_pos);

        // TODO: Handle lighting campfires/candles (set LIT=true)

        // Check if fire can be placed
        if !Self::can_place_fire(context, &fire_pos) {
            return InteractionResult::Fail;
        }

        // Place fire
        let fire_state = vanilla_blocks::FIRE.default_state();
        if !context
            .world
            .set_block(fire_pos, fire_state, UpdateFlags::UPDATE_ALL)
        {
            return InteractionResult::Fail;
        }

        // Play sound (exclude player - they hear it client-side)
        let pitch = 0.8 + rand::random::<f32>() * 0.4; // 0.8 to 1.2
        context.world.play_sound(
            sound_events::ITEM_FLINTANDSTEEL_USE,
            SoundSource::Blocks,
            fire_pos,
            1.0,
            pitch,
            Some(context.player.id),
        );

        // Damage the item (unless creative)
        if !context.player.has_infinite_materials() {
            context.item_stack.hurt_and_break(1, false);
        }

        InteractionResult::Success
    }
}
