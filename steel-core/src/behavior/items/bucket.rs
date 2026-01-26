//! Bucket item behavior implementations.

use std::ptr;

use steel_registry::REGISTRY;
use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::BlockStateProperties;
use steel_registry::items::ItemRef;
use steel_registry::vanilla_blocks;
use steel_registry::vanilla_items;
use steel_utils::types::UpdateFlags;
use steel_utils::math::Vector3;
use steel_utils::BlockStateId;

use crate::player::Player;
use crate::entity::LivingEntity;
use crate::behavior::ItemBehavior;
use crate::behavior::context::InteractionResult;

/// Computes the start (eye position) and end positions for a raytrace.
fn get_ray_endpoints(player: &Player) -> (Vector3<f64>, Vector3<f64>) {
    let start_pos = player.eye_position();
    let (yaw, pitch) = player.rotation();
    let (yaw_rad, pitch_rad) = (f64::from(yaw.to_radians()), f64::from(pitch.to_radians()));
    let block_interaction_range = 4.5; // This is not the same as the block_interaction_range in the
    // player entity.
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
    // Only water and lava have the LEVEL property
    if !ptr::eq(block, vanilla_blocks::WATER) && !ptr::eq(block, vanilla_blocks::LAVA) {
        return false;
    }
    
    // Source blocks have level 0
    state.try_get_value(&BlockStateProperties::LEVEL)
        .map_or(false, |level: u8| level == 0)
}

/// Behavior for filled bucket items (water bucket, lava bucket, etc.)
///
/// When used on a block, places the fluid at the target position and
/// replaces the bucket with an empty bucket.
pub struct FilledBucketBehavior {
    /// The fluid block to place.
    fluid_block: BlockRef,
    /// The empty bucket item to give back.
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
        // Raytrace to find target block, passing through all fluids and air
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

        // Determine placement position: if replaceable, place there; otherwise place adjacent
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

        // If the same fluid already exists, consume the bucket but don't place again
        if ptr::eq(existing_block, self.fluid_block) {
            if !context.player.has_infinite_materials() {
                context.item_stack.set_item(&self.empty_bucket.key);
            }
            return InteractionResult::Success;
        }

        let fluid_state = self.fluid_block.default_state();
        if !context.world.set_block(place_pos, fluid_state, UpdateFlags::UPDATE_ALL_IMMEDIATE) {
            return InteractionResult::Fail;
        }

        // Schedule a fluid tick to trigger spreading
        // Water tick delay = 5, Lava = 30
        let tick_delay = if ptr::eq(self.fluid_block, vanilla_blocks::WATER) { 5 } else { 30 };
        context.world.schedule_fluid_tick(place_pos, context.world.game_time(), tick_delay);

        if !context.player.has_infinite_materials() {
            context.item_stack.set_item(&self.empty_bucket.key);
        }
        // TODO: Play bucket empty sound

        InteractionResult::Success
    }
}

/// Behavior for empty bucket items.
///
/// When used on a fluid source block, picks up the fluid and gives a filled bucket.
/// Only source blocks (level == 0) can be picked up.
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
        
        // Raytrace: stop on source fluids or solid blocks, pass through air and flowing fluids
        let (hit_block, hit_dir) = context.world.raytrace(start, end, |pos, world| {
            let state = world.get_block_state(pos);
            let block = state.get_block();
            
            // Pass through air
            if ptr::eq(block, vanilla_blocks::AIR) {
                return false;
            }
            
            // Check if it's a fluid
            if ptr::eq(block, vanilla_blocks::WATER) || ptr::eq(block, vanilla_blocks::LAVA) {
                // Only stop on source blocks (level == 0)
                return is_source_fluid(state, block);
            }
            
            // Stop on solid/other blocks
            true
        });

        let (hit_pos, _) = match (hit_block, hit_dir) {
            (Some(pos), Some(dir)) => (pos, dir),
            _ => return InteractionResult::Fail,
        };

        let fluid_state = context.world.get_block_state(&hit_pos);
        let fluid_block = fluid_state.get_block();

        // Determine which filled bucket to give based on the fluid type
        let (filled_bucket, tick_delay) = if ptr::eq(fluid_block, vanilla_blocks::WATER) && is_source_fluid(fluid_state, fluid_block) {
            (&vanilla_items::ITEMS.water_bucket, 5u32)
        } else if ptr::eq(fluid_block, vanilla_blocks::LAVA) && is_source_fluid(fluid_state, fluid_block) {
            (&vanilla_items::ITEMS.lava_bucket, 30u32)
        } else {
            // Not a pickable fluid (either not fluid or not source)
            return InteractionResult::Fail;
        };

        // Remove the fluid block (replace with air)
        // TODO: Handle waterlogged blocks when implemented
        let air_state = REGISTRY.blocks.get_default_state_id(vanilla_blocks::AIR);
        if !context.world.set_block(hit_pos, air_state, UpdateFlags::UPDATE_ALL_IMMEDIATE) {
            return InteractionResult::Fail;
        }

        // Schedule fluid ticks for neighboring blocks so they can recalculate
        // This triggers the "de-propagation" - flowing water without a source will disappear
        //let current_tick = context.world.game_time();
        //for offset in [(0, 1, 0), (0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)] {
        //    let neighbor = hit_pos.offset(offset.0, offset.1, offset.2);
        //    context.world.schedule_fluid_tick(neighbor, current_tick, tick_delay);
        //}

        // Give filled bucket (unless creative mode)
        if !context.player.has_infinite_materials() {
            context.item_stack.set_item(&filled_bucket.key);
        }
        
        // TODO: Play bucket fill sound
        InteractionResult::Success
    }
}
