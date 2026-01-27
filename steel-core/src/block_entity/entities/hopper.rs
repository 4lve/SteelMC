//! Hopper block entity implementation.
//!
//! Based on: [HopperBlockEntity.java](file:///home/coco/Documents/GitHub/SteelMC/minecraft-src/minecraft/src/net/minecraft/world/level/block/entity/HopperBlockEntity.java)

use std::any::Any;
use std::sync::{Arc, Weak};

use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use simdnbt::{FromNbtTag, ToNbtTag};
use steel_registry::REGISTRY;
use steel_registry::block_entity_type::BlockEntityTypeRef;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::{BlockStateProperties, Direction};
use steel_registry::data_components::DataComponentPatch;
use steel_registry::item_stack::ItemStack;
use steel_registry::vanilla_block_entity_types;
use steel_utils::{BlockPos, BlockStateId, Identifier};

use crate::block_entity::BlockEntity;
use crate::inventory::container::Container;
use crate::world::World;

/// Block entity for hoppers.
pub struct HopperBlockEntity {
    level: Weak<World>,
    pos: BlockPos,
    state: BlockStateId,
    removed: bool,
    items: [ItemStack; 5],
    ticked_game_time: i64,
    transfer_cooldown: i32,
}

impl HopperBlockEntity {
    /// Creates a new hopper block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        Self {
            level,
            pos,
            state,
            removed: false,
            items: [
                ItemStack::empty(),
                ItemStack::empty(),
                ItemStack::empty(),
                ItemStack::empty(),
                ItemStack::empty(),
            ],
            ticked_game_time: 0,
            transfer_cooldown: -1,
        }
    }

    /// Tries to move items in or out of the hopper.
    // Based on HopperBlockEntity.tryMoveItems (line 297)
    fn try_move_items(&mut self, world: &World) -> bool {
        if self.is_on_cooldown() {
            return false;
        }

        let mut result = false;

        if !self.is_empty() {
            result = self.eject_items(world);
        }

        if !self.inventory_full() {
            result |= self.suck_in_items(world);
        }

        if result {
            self.set_cooldown(8);
            self.set_changed();
        }

        result
    }

    fn inventory_full(&self) -> bool {
        self.items.iter().all(|item| !item.is_empty())
    }

    /// Ejects items into the container the hopper is facing.
    // Based on HopperBlockEntity.ejectItems (line 330)
    fn eject_items(&mut self, world: &World) -> bool {
        let facing = self.state.get_value(&BlockStateProperties::FACING_HOPPER);
        let (dx, dy, dz) = facing.offset();
        let target_pos = self.pos.offset(dx, dy, dz);

        // Simple check: is there a container at target_pos?
        // In a full implementation, we'd need to handle inventories dynamically.
        // For now, we reuse the pattern of locking the target block entity if it exists.

        let Some(target_be) = world.get_block_entity(&target_pos) else {
            return false;
        };

        let mut target_guard = target_be.lock();
        let Some(target_container) = target_guard.as_container_mut() else {
            return false;
        };

        // Iterate through our slots to find one to push
        for i in 0..self.items.len() {
            if !self.items[i].is_empty() {
                let mut stack = self.remove_item(i, 1);
                if stack.is_empty() {
                    continue;
                }

                // Try to add to target
                if target_container.add(&mut stack) {
                    // Successfully added (stack is now empty or reduced, but add() returns true if *everything* added?)
                    // Container.add returns true if *entire* stack was added.
                    // If it returned true, stack is empty.
                    // If false, stack contains remainder.

                    // If we have remainder (shouldn't happen with count 1 unless full)
                    // If we removed 1, and add failed (returned false), we might have remainder 1.
                    // If add returned true, remainder is 0.

                    if !stack.is_empty() {
                        // Put it back?
                        self.items[i].grow(stack.count());
                        return false; // Failed to push
                    }

                    // Success
                    target_container.set_changed();
                    return true;
                } else {
                    // Failed to add completely. Return item.
                    self.items[i].grow(stack.count());
                }
            }
        }

        false
    }

    /// Sucks in items from above.
    // Based on HopperBlockEntity.suckInItems (line 396)
    fn suck_in_items(&mut self, world: &World) -> bool {
        let (dx, dy, dz) = Direction::Up.offset();
        let source_pos = self.pos.offset(dx, dy, dz);

        // 1. Try to pull from container above
        if let Some(source_be) = world.get_block_entity(&source_pos) {
            let mut source_guard = source_be.lock();
            if let Some(source_container) = source_guard.as_container_mut() {
                // Iterate slots in source
                for i in 0..source_container.get_container_size() {
                    let item = source_container.get_item(i).clone();
                    if !item.is_empty() {
                        // Try to take 1 item
                        let mut one_item = item.clone();
                        one_item.set_count(1);

                        // Try to add to self
                        if self.add(&mut one_item) {
                            // Success, remove from source
                            source_container.remove_item(i, 1);
                            source_container.set_changed();
                            return true;
                        }
                    }
                }
            }
        }

        // 2. TODO: Pull item entities from the world.
        // Needs AABB checks and Entity iteration which isn't fully exposed in World yet.

        false
    }

    fn set_cooldown(&mut self, cooldown: i32) {
        self.transfer_cooldown = cooldown;
    }

    fn is_on_cooldown(&self) -> bool {
        self.transfer_cooldown > 0
    }
}

impl BlockEntity for HopperBlockEntity {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_type(&self) -> BlockEntityTypeRef {
        vanilla_block_entity_types::HOPPER
    }

    fn get_block_pos(&self) -> BlockPos {
        self.pos
    }

    fn get_block_state(&self) -> BlockStateId {
        self.state
    }

    fn set_block_state(&mut self, state: BlockStateId) {
        self.state = state;
    }

    fn is_removed(&self) -> bool {
        self.removed
    }

    fn set_removed(&mut self) {
        self.removed = true;
    }

    fn clear_removed(&mut self) {
        self.removed = false;
    }

    fn get_level(&self) -> Option<Arc<World>> {
        self.level.upgrade()
    }

    fn load_additional(&mut self, nbt: &BorrowedNbtCompound<'_>) {
        // Convert to NbtCompound view
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();

        // Load items
        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                // Each item has a "Slot" byte and item data
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < self.items.len() {
                        // Parse item directly from the borrowed compound
                        if let Some(item) = item_from_borrowed_compound(&compound) {
                            self.items[slot] = item;
                        }
                    }
                }
            }
        }

        self.transfer_cooldown = nbt_view.int("TransferCooldown").unwrap_or(0);
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        // Save items to NBT (only non-empty slots)
        let mut items: Vec<NbtCompound> = Vec::new();
        for (slot, item) in self.items.iter().enumerate() {
            if !item.is_empty() {
                // Use ItemStack's ToNbtTag implementation for proper component serialization
                if let NbtTag::Compound(mut item_nbt) = item.clone().to_nbt_tag() {
                    item_nbt.insert("Slot", slot as i8);
                    items.push(item_nbt);
                }
            }
        }
        nbt.insert("Items", NbtList::Compound(items));

        nbt.insert("TransferCooldown", self.transfer_cooldown);
    }

    fn is_ticking(&self) -> bool {
        true
    }

    fn tick(&mut self, world: &World) {
        self.transfer_cooldown -= 1;
        self.ticked_game_time = world.level_data.read().data().game_time;

        if !self.is_on_cooldown() && self.state.get_value(&BlockStateProperties::ENABLED) {
            self.try_move_items(world);
        }
    }

    fn as_container(&self) -> Option<&(dyn Container + 'static)> {
        Some(self)
    }

    fn as_container_mut(&mut self) -> Option<&mut (dyn Container + 'static)> {
        Some(self)
    }
}

impl Container for HopperBlockEntity {
    fn get_container_size(&self) -> usize {
        5
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        &self.items[slot]
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        &mut self.items[slot]
    }

    fn set_item(&mut self, slot: usize, stack: ItemStack) {
        self.items[slot] = stack;
        let max_stack = self.get_max_stack_size();
        if self.items[slot].count() > max_stack {
            self.items[slot].set_count(max_stack);
        }
        self.set_changed();
    }

    fn set_changed(&mut self) {
        BlockEntity::set_changed(self);
    }
}

/// Parses an `ItemStack` from a borrowed `NbtCompound`.
fn item_from_borrowed_compound(compound: &NbtCompoundView<'_, '_>) -> Option<ItemStack> {
    // Get the item ID
    let id_str = compound.string("id")?.to_str();
    let id = id_str.parse::<Identifier>().ok()?;

    // Look up the item in the registry
    let item = REGISTRY.items.by_key(&id)?;

    // Get the count (default to 1 if not present)
    let count = compound.int("count").unwrap_or(1);

    // Parse components if present
    let patch = compound
        .get("components")
        .and_then(DataComponentPatch::from_nbt_tag)
        .unwrap_or_default();

    Some(ItemStack::with_count_and_patch(item, count, patch))
}
