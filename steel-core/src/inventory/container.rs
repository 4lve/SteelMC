//! Container trait for item storage.

use steel_registry::item_stack::ItemStack;

/// A trait for objects that can store items in slots.
///
/// This is the base abstraction for anything that holds items: chests, furnaces,
/// player inventories, hoppers, etc.
pub trait Container: Send + Sync {
    /// Returns the number of slots in this container.
    fn size(&self) -> usize;

    /// Returns true if all slots are empty.
    fn is_empty(&self) -> bool {
        for i in 0..self.size() {
            if !self.get_item(i).is_empty() {
                return false;
            }
        }
        true
    }

    /// Gets the item in the given slot.
    fn get_item(&self, slot: usize) -> &ItemStack;

    /// Gets a mutable reference to the item in the given slot.
    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack;

    /// Removes up to `count` items from the given slot and returns them.
    fn remove_item(&mut self, slot: usize, count: i32) -> ItemStack {
        let item = self.get_item_mut(slot);
        if item.is_empty() || count <= 0 {
            return ItemStack::empty();
        }
        let result = item.split(count);
        if !result.is_empty() {
            self.set_changed();
        }
        result
    }

    /// Removes and returns the entire item stack from the given slot without triggering updates.
    fn remove_item_no_update(&mut self, slot: usize) -> ItemStack {
        let item = self.get_item_mut(slot);
        item.copy_and_clear()
    }

    /// Sets the item in the given slot.
    fn set_item(&mut self, slot: usize, item: ItemStack);

    /// Returns the maximum stack size this container allows.
    fn max_stack_size(&self) -> i32 {
        99
    }

    /// Returns the maximum stack size for a specific item in this container.
    fn max_stack_size_for(&self, item: &ItemStack) -> i32 {
        self.max_stack_size().min(item.max_stack_size())
    }

    /// Called when the container contents change.
    fn set_changed(&mut self);

    /// Clears all items from this container.
    fn clear(&mut self) {
        for i in 0..self.size() {
            self.set_item(i, ItemStack::empty());
        }
    }

    /// Returns whether a given item can be placed in the given slot.
    fn can_place_item(&self, _slot: usize, _item: &ItemStack) -> bool {
        true
    }

    /// Returns whether an item can be taken from the given slot.
    fn can_take_item(&self, _slot: usize, _item: &ItemStack) -> bool {
        true
    }
}
