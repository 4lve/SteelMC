//! Hopper block behavior implementation.
//!
//! Based on: [HopperBlock.java](file:///home/coco/Documents/GitHub/SteelMC/minecraft-src/minecraft/src/net/minecraft/world/level/block/HopperBlock.java)

use std::sync::Weak;

use steel_registry::blocks::BlockRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{BlockStateProperties, Direction};
use steel_registry::vanilla_block_entity_types;
use steel_utils::text::TextComponent;
use steel_utils::{BlockPos, BlockStateId, translations};

use crate::behavior::block::BlockBehaviour;
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::{BLOCK_ENTITIES, SharedBlockEntity};
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::hopper_menu::HopperMenuProvider;
use crate::inventory::lock::ContainerRef;
use crate::player::Player;
use crate::world::World;

/// Behavior for hopper blocks.
pub struct HopperBlock {
    block: BlockRef,
}

impl HopperBlock {
    /// Creates a new hopper block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehaviour for HopperBlock {
    fn update_shape(
        &self,
        state: BlockStateId,
        _world: &World,
        _pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        match direction {
            Direction::Down => {
                // If the block below changes, we might need to update shape?
                // Vanilla HopperBlock doesn't override updateShape for shape updates,
                // but it does for attachment checks.
                // For now, simple standard behavior.
                state
            }
            _ => state,
        }
    }

    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // Based on HopperBlock.getStateForPlacement (line 125)
        // Direction direction = context.getClickedFace().getOpposite();
        // if (direction == Direction.UP) { direction = Direction.DOWN; }

        let mut facing = context.clicked_face.opposite();
        if facing == Direction::Up {
            facing = Direction::Down;
        }

        let enabled = true; // Default enabled

        Some(
            self.block
                .default_state()
                .set_value(&BlockStateProperties::FACING_HOPPER, facing)
                .set_value(&BlockStateProperties::ENABLED, enabled),
        )
    }

    fn use_without_item(
        &self,
        _state: BlockStateId,
        world: &World,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
    ) -> InteractionResult {
        // based on HopperBlock.useWithoutItem (line 155)

        let Some(block_entity) = world.get_block_entity(&pos) else {
            return InteractionResult::Pass;
        };

        let Some(container_ref) = ContainerRef::from_block_entity(block_entity) else {
            return InteractionResult::Pass;
        };

        player.open_menu(&HopperMenuProvider::new(
            player.inventory.clone(),
            container_ref,
            TextComponent::new().translate(translations::CONTAINER_HOPPER.msg()),
        ));

        // TODO: Award stat INSPECT_HOPPER
        InteractionResult::Success
    }

    fn has_block_entity(&self) -> bool {
        true
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> Option<SharedBlockEntity> {
        BLOCK_ENTITIES.create(vanilla_block_entity_types::HOPPER, level, pos, state)
    }

    fn has_analog_output_signal(&self, _state: BlockStateId) -> bool {
        true
    }

    fn get_analog_output_signal(&self, _state: BlockStateId, world: &World, pos: BlockPos) -> i32 {
        world.get_block_entity(&pos).map_or(0, |be| {
            let guard = be.lock();
            if let Some(container) = guard.as_container() {
                calculate_redstone_signal_from_container(container)
            } else {
                0
            }
        })
    }

    fn on_place(
        &self,
        _state: BlockStateId,
        _world: &World,
        _pos: BlockPos,
        _old_state: BlockStateId,
        _moved_by_piston: bool,
    ) {
        // Check for power on placement
        // In vanilla (line 134): if (!world.isClientSide) { this.checkPoweredState(world, pos, state); }
        // checkPoweredState checks if powered, and if so, sets ENABLED to false.

        // Since we don't have has_neighbor_signal fully working/exposed in the same way yet,
        // we'll leave this as a TODO or implement what we can.
        // Assuming world.has_neighbor_signal(pos) exists or similar.

        // Temporary placeholder:
        // let is_powered = false; // world.has_neighbor_signal(pos);
        // if is_powered {
        //      let new_state = state.set_value(&BlockStateProperties::ENABLED, false);
        //      world.set_block(pos, new_state, steel_utils::types::UpdateFlags::UPDATE_ALL);
        // }
    }

    fn handle_neighbor_changed(
        &self,
        _state: BlockStateId,
        _world: &World,
        _pos: BlockPos,
        _source_block: BlockRef,
        _moved_by_piston: bool,
    ) {
        // Based on HopperBlock.neighborChanged (line 140)
        // boolean flag = !world.hasNeighborSignal(pos);
        // if (flag != state.getValue(ENABLED)) {
        //    world.setBlock(pos, state.setValue(ENABLED, Boolean.valueOf(flag)), 4);
        // }

        // Similar placeholder for signaled state
        /*
        let enabled = !world.has_neighbor_signal(pos);
        if enabled != state.get_value(&BlockStateProperties::ENABLED) {
            let new_state = state.set_value(&BlockStateProperties::ENABLED, enabled);
            world.set_block(pos, new_state, 4);
        }
        */
    }
}
