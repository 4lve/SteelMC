//! Bucket item behavior implementations.
//!
//! Handles water buckets, lava buckets, and empty buckets.
//! Based on vanilla Minecraft's BucketItem.
//!
// TODO: Add support for bucket stacks (count > 1) without deadlocks
// TODO: Play bucket sounds (fill/empty)
// TODO: Spawn particles
// TODO: Handle waterlogging

use std::ptr;

use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::BlockStateProperties;
use steel_registry::blocks::BlockRef;
use steel_registry::items::ItemRef;
use steel_registry::sound_events;
use steel_registry::vanilla_blocks;
use steel_registry::vanilla_items;
use steel_registry::REGISTRY;
use steel_utils::math::Vector3;
use steel_utils::types::UpdateFlags;
use steel_utils::BlockStateId;

use crate::behavior::context::InteractionResult;
use crate::behavior::ItemBehavior;
use crate::entity::LivingEntity;
use crate::player::Player;

/// Computes the start (eye position) and end positions for a raytrace.
fn get_ray_endpoints(player: &Player) -> (Vector3<f64>, Vector3<f64>) {
    let start_pos = player.eye_position();
    let (yaw, pitch) = player.rotation();
    let (yaw_rad, pitch_rad) = (f64::from(yaw.to_radians()), f64::from(pitch.to_radians()));
    let block_interaction_range = 4.5;
    let direction = Vector3::new(
        -yaw_rad.sin() * pitch_rad.cos() * block_interaction_range,
        -pitch_rad.sin() * block_interaction_range,
        pitch_rad.cos() * yaw_rad.cos() * block_interaction_range,
    );

    let end_pos = start_pos.add(&direction);
    (start_pos, end_pos)
}

/// Checks if a fluid block state is a source block (level == 0).
fn is_source_fluid(state: BlockStateId, block: BlockRef) -> bool {
    if !ptr::eq(block, vanilla_blocks::WATER) && !ptr::eq(block, vanilla_blocks::LAVA) {
        return false;
    }

    state
        .try_get_value(&BlockStateProperties::LEVEL)
        .map_or(false, |level: u8| level == 0)
}

/// Behavior for filled bucket items (water bucket, lava bucket)
///
/// Places fluid and gives back empty bucket.
/// NOTE: Stack support (count > 1) is not yet implemented to avoid deadlocks.
pub struct FilledBucketBehavior {
    fluid_block: BlockRef,
    empty_bucket: ItemRef,
}

impl FilledBucketBehavior {
    /// Creates a new filled bucket behavior.
    #[must_use]
    pub const fn new(fluid_block: BlockRef, empty_bucket: ItemRef) -> Self {
        Self {
            fluid_block,
            empty_bucket,
        }
    }
}

impl ItemBehavior for FilledBucketBehavior {
    fn use_item(&self, context: &mut crate::behavior::UseItemContext) -> InteractionResult {
        // Raytrace to find target block
        let (start, end) = get_ray_endpoints(context.player);
        let (ray_block, ray_dir) = context.world.raytrace(start, end, |pos, world| {
            let state = world.get_block_state(pos);
            let block = state.get_block();
            // Pass through air and all fluids
            if ptr::eq(block, vanilla_blocks::AIR)
                || ptr::eq(block, vanilla_blocks::WATER)
                || ptr::eq(block, vanilla_blocks::LAVA)
            {
                return false;
            }
            true
        });

        let (clicked_pos, direction) = match (ray_block, ray_dir) {
            (Some(pos), Some(dir)) => (pos, dir),
            _ => return InteractionResult::Fail,
        };

        let clicked_state = context.world.get_block_state(&clicked_pos);
        let clicked_block = clicked_state.get_block();

        let place_pos = if clicked_block.config.replaceable {
            clicked_pos
        } else {
            direction.relative(&clicked_pos)
        };

        if !context.world.is_in_valid_bounds(&place_pos) {
            return InteractionResult::Fail;
        }

        let existing_state = context.world.get_block_state(&place_pos);
        let existing_block = existing_state.get_block();

        if !existing_block.config.replaceable {
            return InteractionResult::Fail;
        }

        // If same fluid already exists and is source, just consume bucket
        if ptr::eq(existing_block, self.fluid_block)
            && is_source_fluid(existing_state, existing_block)
        {
            if !context.player.has_infinite_materials() {
                context.item_stack.set_item(&self.empty_bucket.key);
            }
            return InteractionResult::Success;
        }

        // Place fluid
        let fluid_state = self.fluid_block.default_state();
        if !context
            .world
            .set_block(place_pos, fluid_state, UpdateFlags::UPDATE_ALL_IMMEDIATE)
        {
            return InteractionResult::Fail;
        }

        // Schedule tick for spreading
        let tick_delay = if ptr::eq(self.fluid_block, vanilla_blocks::WATER) {
            5
        } else {
            30
        };

        let current_tick = context.world.game_time();
        context
            .world
            .schedule_fluid_tick(place_pos, current_tick, tick_delay);

        // Play bucket empty sound
        let sound_id = if ptr::eq(self.fluid_block, vanilla_blocks::WATER) {
            sound_events::ITEM_BUCKET_EMPTY
        } else {
            sound_events::ITEM_BUCKET_EMPTY_LAVA
        };
        context
            .world
            .play_block_sound(sound_id, place_pos, 1.0, 1.0, None);

        // Replace with empty bucket
        if !context.player.has_infinite_materials() {
            context.item_stack.set_item(&self.empty_bucket.key);
        }

        InteractionResult::Success
    }
}

/// Behavior for empty bucket items.
///
/// Picks up fluid from source blocks and gives filled bucket.
/// NOTE: Stack support (count > 1) is not yet implemented to avoid deadlocks.
pub struct EmptyBucketBehavior;

impl EmptyBucketBehavior {
    /// Creates a new empty bucket behavior.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl ItemBehavior for EmptyBucketBehavior {
    fn use_item(&self, context: &mut crate::behavior::UseItemContext) -> InteractionResult {
        let (start, end) = get_ray_endpoints(context.player);

        // Raytrace: stop on source fluids
        let (hit_block, hit_dir) = context.world.raytrace(start, end, |pos, world| {
            let state = world.get_block_state(pos);
            let block = state.get_block();

            if ptr::eq(block, vanilla_blocks::AIR) {
                return false;
            }

            // Check for direct fluid blocks
            if ptr::eq(block, vanilla_blocks::WATER) || ptr::eq(block, vanilla_blocks::LAVA) {
                return is_source_fluid(state, block);
            }

            // Check for waterlogged blocks
            if let Some(true) = state.try_get_value(&BlockStateProperties::WATERLOGGED) {
                return true;
            }

            true
        });

        let (hit_pos, _) = match (hit_block, hit_dir) {
            (Some(pos), Some(dir)) => (pos, dir),
            _ => return InteractionResult::Fail,
        };

        let fluid_state = context.world.get_block_state(&hit_pos);
        let fluid_block = fluid_state.get_block();

        // Determine filled bucket type
        let (filled_bucket, is_waterlogged) = if ptr::eq(fluid_block, vanilla_blocks::WATER)
            && is_source_fluid(fluid_state, fluid_block)
        {
            (&vanilla_items::ITEMS.water_bucket, false)
        } else if ptr::eq(fluid_block, vanilla_blocks::LAVA)
            && is_source_fluid(fluid_state, fluid_block)
        {
            (&vanilla_items::ITEMS.lava_bucket, false)
        } else if let Some(true) = fluid_state.try_get_value(&BlockStateProperties::WATERLOGGED) {
            (&vanilla_items::ITEMS.water_bucket, true)
        } else {
            return InteractionResult::Fail;
        };

        let tick_delay = 5; // Default water delay

        // Remove fluid
        let success = if is_waterlogged {
            // Un-waterlog the block
            let new_state = fluid_state.set_value(&BlockStateProperties::WATERLOGGED, false);
            context
                .world
                .set_block(hit_pos, new_state, UpdateFlags::UPDATE_ALL_IMMEDIATE)
        } else {
            // Remove liquid block
            let air_state = REGISTRY.blocks.get_default_state_id(vanilla_blocks::AIR);
            context
                .world
                .set_block(hit_pos, air_state, UpdateFlags::UPDATE_ALL_IMMEDIATE)
        };

        if !success {
            return InteractionResult::Fail;
        }

        // Schedule ticks for de-propagation
        let current_tick = context.world.game_time();

        // IMPORTANT: Schedule ticks for neighbors so infinite sources regenerate
        // TODO: Visual sync issue - client sees air before regenerated water
        // This might need force-sending block updates to the specific player
        for offset in [
            (0, 1, 0),
            (0, -1, 0),
            (1, 0, 0),
            (-1, 0, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            let neighbor = hit_pos.offset(offset.0, offset.1, offset.2);
            context
                .world
                .schedule_fluid_tick(neighbor, current_tick, tick_delay);
        }

        // Play bucket fill sound
        let sound_id = if ptr::eq(fluid_block, vanilla_blocks::WATER) {
            sound_events::ITEM_BUCKET_FILL
        } else {
            sound_events::ITEM_BUCKET_FILL_LAVA
        };
        context
            .world
            .play_block_sound(sound_id, hit_pos, 1.0, 1.0, None);

        // Give filled bucket
        if !context.player.has_infinite_materials() {
            context.item_stack.set_item(&filled_bucket.key);
        }

        InteractionResult::Success
    }
}
