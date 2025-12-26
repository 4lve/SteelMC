//! Player inventory implementation.

use steel_registry::item_stack::ItemStack;

use super::Container;

/// The number of main inventory slots (excluding armor and offhand).
pub const INVENTORY_SIZE: usize = 36;
/// The number of hotbar slots.
pub const HOTBAR_SIZE: usize = 9;
/// The offhand slot index.
pub const SLOT_OFFHAND: usize = 40;

/// The player's inventory.
///
/// Contains 36 main slots (0-35), where slots 0-8 are the hotbar.
/// Equipment slots (armor, offhand) are handled separately by the player entity.
#[derive(Debug)]
pub struct PlayerInventory {
    /// The 36 main inventory slots.
    items: [ItemStack; INVENTORY_SIZE],
    /// The currently selected hotbar slot (0-8).
    selected_slot: usize,
    /// Tracks whether the inventory has been modified.
    times_changed: u32,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerInventory {
    /// Creates a new empty player inventory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: std::array::from_fn(|_| ItemStack::empty()),
            selected_slot: 0,
            times_changed: 0,
        }
    }

    /// Returns the currently selected hotbar slot index (0-8).
    #[must_use]
    pub fn selected_slot(&self) -> usize {
        self.selected_slot
    }

    /// Sets the selected hotbar slot.
    ///
    /// # Panics
    /// Panics if `slot` is not in the range 0-8.
    pub fn set_selected_slot(&mut self, slot: usize) {
        assert!(Self::is_hotbar_slot(slot), "Invalid hotbar slot: {slot}");
        self.selected_slot = slot;
    }

    /// Returns the currently selected item (item in main hand).
    #[must_use]
    pub fn selected_item(&self) -> &ItemStack {
        &self.items[self.selected_slot]
    }

    /// Returns a mutable reference to the selected item.
    #[must_use]
    pub fn selected_item_mut(&mut self) -> &mut ItemStack {
        &mut self.items[self.selected_slot]
    }

    /// Returns whether the given slot is a hotbar slot (0-8).
    #[must_use]
    pub const fn is_hotbar_slot(slot: usize) -> bool {
        slot < HOTBAR_SIZE
    }

    /// Returns the number of times the inventory has been modified.
    #[must_use]
    pub fn times_changed(&self) -> u32 {
        self.times_changed
    }

    /// Finds the first empty slot, or -1 if none.
    #[must_use]
    pub fn get_free_slot(&self) -> Option<usize> {
        self.items.iter().position(ItemStack::is_empty)
    }

    /// Finds a slot containing an item that matches the given stack.
    #[must_use]
    pub fn find_slot_matching_item(&self, item: &ItemStack) -> Option<usize> {
        self.items.iter().position(|slot_item| {
            !slot_item.is_empty() && ItemStack::is_same_item_same_components(slot_item, item)
        })
    }

    /// Finds a slot with remaining space for the given item.
    #[must_use]
    pub fn get_slot_with_remaining_space(&self, item: &ItemStack) -> Option<usize> {
        // First check the selected slot
        if self.has_remaining_space_for(&self.items[self.selected_slot], item) {
            return Some(self.selected_slot);
        }

        // Then check all slots
        self.items
            .iter()
            .position(|slot_item| self.has_remaining_space_for(slot_item, item))
    }

    /// Returns whether a slot has remaining space for an item.
    fn has_remaining_space_for(&self, slot_item: &ItemStack, new_item: &ItemStack) -> bool {
        !slot_item.is_empty()
            && ItemStack::is_same_item_same_components(slot_item, new_item)
            && slot_item.is_stackable()
            && slot_item.count() < self.max_stack_size_for(slot_item)
    }

    /// Finds a suitable hotbar slot for placing a new item.
    #[must_use]
    pub fn get_suitable_hotbar_slot(&self) -> usize {
        // First pass: look for empty slots starting from selected
        for offset in 0..HOTBAR_SIZE {
            let index = (self.selected_slot + offset) % HOTBAR_SIZE;
            if self.items[index].is_empty() {
                return index;
            }
        }

        // Second pass: return selected slot as fallback
        self.selected_slot
    }

    /// Tries to add an item to the inventory.
    ///
    /// Returns `true` if at least some items were added.
    pub fn add(&mut self, item: &mut ItemStack) -> bool {
        self.add_to_slot(None, item)
    }

    /// Tries to add an item to a specific slot, or any slot if `None`.
    pub fn add_to_slot(&mut self, slot: Option<usize>, item: &mut ItemStack) -> bool {
        if item.is_empty() {
            return false;
        }

        let original_count = item.count();

        // For damaged items, place in a free slot directly
        // (Minecraft doesn't stack damaged items into existing stacks)
        if let Some(target_slot) = slot {
            self.add_resource_to_slot(target_slot, item);
        } else {
            // Try to merge with existing stacks first, then use empty slots
            loop {
                let last_count = item.count();
                self.add_resource(item);
                if item.is_empty() || item.count() >= last_count {
                    break;
                }
            }
        }

        item.count() < original_count
    }

    /// Adds items to the inventory, preferring existing stacks.
    fn add_resource(&mut self, item: &mut ItemStack) {
        // First try to find a slot with remaining space
        if let Some(slot) = self.get_slot_with_remaining_space(item) {
            self.add_resource_to_slot(slot, item);
            return;
        }

        // Otherwise, find an empty slot
        if let Some(slot) = self.get_free_slot() {
            self.add_resource_to_slot(slot, item);
        }
    }

    /// Adds items to a specific slot.
    fn add_resource_to_slot(&mut self, slot: usize, item: &mut ItemStack) {
        let max_size = self.max_stack_size_for(item);
        let slot_item = &mut self.items[slot];

        if slot_item.is_empty() {
            // Empty slot: place items up to max stack size
            let to_add = item.count().min(max_size);
            *slot_item = item.copy_with_count(to_add);
            item.shrink(to_add);
            self.set_changed();
        } else if ItemStack::is_same_item_same_components(slot_item, item) {
            // Existing compatible stack: merge
            let space = max_size - slot_item.count();
            let to_add = item.count().min(space);
            if to_add > 0 {
                slot_item.grow(to_add);
                item.shrink(to_add);
                self.set_changed();
            }
        }
    }

    /// Places an item back in the inventory, dropping if no space.
    ///
    /// Returns `true` if the item was fully placed.
    pub fn place_item_back(&mut self, item: &mut ItemStack) -> bool {
        while !item.is_empty() {
            let slot = self
                .get_slot_with_remaining_space(item)
                .or_else(|| self.get_free_slot());

            match slot {
                Some(slot_idx) => {
                    let space = item.max_stack_size() - self.items[slot_idx].count();
                    let to_add = item.count().min(space);
                    let split = item.split(to_add);
                    self.add_to_slot(Some(slot_idx), &mut split.clone());
                }
                None => return false, // No space, caller should drop the item
            }
        }
        true
    }

    /// Drops all items from the inventory, returning them.
    pub fn drop_all(&mut self) -> Vec<ItemStack> {
        let mut dropped = Vec::new();
        for item in &mut self.items {
            if !item.is_empty() {
                dropped.push(item.copy_and_clear());
            }
        }
        self.set_changed();
        dropped
    }

    /// Swaps two slots.
    pub fn swap_slots(&mut self, slot_a: usize, slot_b: usize) {
        if slot_a < self.items.len() && slot_b < self.items.len() {
            self.items.swap(slot_a, slot_b);
            self.set_changed();
        }
    }

    /// Removes from the selected slot and returns it.
    pub fn remove_from_selected(&mut self, all: bool) -> ItemStack {
        let count = if all {
            self.items[self.selected_slot].count()
        } else {
            1
        };
        self.remove_item(self.selected_slot, count)
    }
}

impl Container for PlayerInventory {
    fn size(&self) -> usize {
        INVENTORY_SIZE
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        &self.items[slot]
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        &mut self.items[slot]
    }

    fn set_item(&mut self, slot: usize, item: ItemStack) {
        self.items[slot] = item;
        self.set_changed();
    }

    fn set_changed(&mut self) {
        self.times_changed = self.times_changed.wrapping_add(1);
    }

    fn max_stack_size(&self) -> i32 {
        64
    }
}
