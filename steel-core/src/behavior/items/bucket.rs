//! Bucket item behavior implementations.

use std::ptr;

use steel_registry::REGISTRY;
use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::items::ItemRef;
use steel_registry::vanilla_blocks;
use steel_registry::vanilla_items;
use steel_utils::types::UpdateFlags;
use steel_utils::math::Vector3;
use crate::player::Player;

use crate::entity::LivingEntity; 
use crate::behavior::ItemBehavior;
use crate::behavior::context::InteractionResult;

fn get_start_and_end_pos(player: &Player) -> (Vector3<f64>, Vector3<f64>) {
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
        // Raytrace to find target block through fluids
        let (start, end) = get_start_and_end_pos(context.player);
        let (ray_block, ray_dir) = context.world.raytrace(start, end, |pos, world| {
             let state = world.get_block_state(pos);
             let block = state.get_block();
             if ptr::eq(block, vanilla_blocks::WATER) || ptr::eq(block, vanilla_blocks::LAVA) || ptr::eq(block, vanilla_blocks::AIR) {
                 return false;
             }
             true
        });

        let (clicked_pos, direction) = if let (Some(pos), Some(dir)) = (ray_block, ray_dir) {
             (pos, dir)
        } else {
             return InteractionResult::Fail;
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

        if ptr::eq(existing_block, self.fluid_block) {
            return InteractionResult::Pass;
        }

        let fluid_state = self.fluid_block.default_state();
        if !context.world.set_block(place_pos, fluid_state, UpdateFlags::UPDATE_ALL_IMMEDIATE) {
            return InteractionResult::Fail;
        }

        if !context.player.has_infinite_materials() {
            context.item_stack.set_item(&self.empty_bucket.key);
        }
        // TODO: Sound

        InteractionResult::Success
    }
}

/// Behavior for empty bucket items.
///
/// When used on a block, attempts to pick up fluid from that block.
/// Supports picking up water and lava source blocks.
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
        let (start, end) = get_start_and_end_pos(context.player);
        let (hit_block, hit_dir) = context.world.raytrace(start, end, |pos, world| {
            let state = world.get_block_state(pos);
            let block = state.get_block();
            // Stop on fluids or solids (ignore air)
            if ptr::eq(block, vanilla_blocks::AIR) {
                return false;
            }

            // TODO
            // Add check for replaceable blocks
            // Add check for fluids ( stop only on source blocks)
            true 
        });

        if let (Some(hit_pos), Some(_)) = (hit_block, hit_dir) {
            // We hit something (fluid or solid). Check what it is.
            let fluid_state = context.world.get_block_state(&hit_pos);
            let fluid_block = fluid_state.get_block();

            log::info!("EmptyBucket hit block: {}", fluid_block.key);

            let filled_bucket = if ptr::eq(fluid_block, vanilla_blocks::WATER) {
                &vanilla_items::ITEMS.water_bucket
            } else if ptr::eq(fluid_block, vanilla_blocks::LAVA) {
                &vanilla_items::ITEMS.lava_bucket
            } else {
                 return InteractionResult::Fail;
            };

            // Remove the fluid block (replace with air)
            let air_state = REGISTRY.blocks.get_default_state_id(vanilla_blocks::AIR);
            if !context
                .world
                .set_block(hit_pos, air_state, UpdateFlags::UPDATE_ALL_IMMEDIATE)
            {
                return InteractionResult::Fail;
            }

            // Give filled bucket
            if !context.player.has_infinite_materials() {
                context.item_stack.set_item(&filled_bucket.key);
            }
            // TODO: Sound
            InteractionResult::Success
        } else {
            InteractionResult::Fail
        }
    }
}

