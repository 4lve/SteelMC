//! Slot abstraction for container menu slots.

use steel_registry::item_stack::ItemStack;

/// Represents a slot in a container menu.
///
/// A slot wraps a container slot index and provides methods for item manipulation
/// with validation (may_place, may_pickup) and safe operations (safe_insert, safe_take).
#[derive(Debug)]
pub struct Slot {
    /// The index within the container.
    container_slot: usize,
    /// The index within the menu's slot list.
    pub index: usize,
    /// X position for client rendering (not used server-side).
    pub x: i32,
    /// Y position for client rendering (not used server-side).
    pub y: i32,
}

impl Slot {
    /// Creates a new slot.
    #[must_use]
    pub fn new(container_slot: usize, x: i32, y: i32) -> Self {
        Self {
            container_slot,
            index: 0, // Set by the menu when added
            x,
            y,
        }
    }

    /// Returns the container slot index.
    #[must_use]
    pub fn container_slot(&self) -> usize {
        self.container_slot
    }

    /// Returns whether the given item can be placed in this slot.
    ///
    /// Override by using `SlotType` variants for specialized slots.
    #[must_use]
    pub fn may_place(&self, _item: &ItemStack) -> bool {
        true
    }

    /// Returns whether items can be picked up from this slot.
    #[must_use]
    pub fn may_pickup(&self) -> bool {
        true
    }

    /// Returns whether this slot is currently active/visible.
    #[must_use]
    pub fn is_active(&self) -> bool {
        true
    }

    /// Returns the maximum stack size for this slot.
    #[must_use]
    pub fn max_stack_size(&self) -> i32 {
        99
    }

    /// Returns the maximum stack size for a specific item in this slot.
    #[must_use]
    pub fn max_stack_size_for(&self, item: &ItemStack) -> i32 {
        self.max_stack_size().min(item.max_stack_size())
    }
}

/// Operations on slots that require mutable access to the container.
/// These are implemented as standalone functions since slots don't own their container.
pub mod slot_ops {
    use super::*;
    use crate::inventory::Container;

    /// Gets the item in the slot.
    #[must_use]
    pub fn get_item<'a>(slot: &Slot, container: &'a impl Container) -> &'a ItemStack {
        container.get_item(slot.container_slot)
    }

    /// Gets a mutable reference to the item in the slot.
    #[must_use]
    pub fn get_item_mut<'a>(slot: &Slot, container: &'a mut impl Container) -> &'a mut ItemStack {
        container.get_item_mut(slot.container_slot)
    }

    /// Returns whether the slot has an item.
    #[must_use]
    pub fn has_item(slot: &Slot, container: &impl Container) -> bool {
        !get_item(slot, container).is_empty()
    }

    /// Sets the item in the slot.
    pub fn set_item(slot: &Slot, container: &mut impl Container, item: ItemStack) {
        container.set_item(slot.container_slot, item);
    }

    /// Removes up to `amount` items from the slot.
    pub fn remove(slot: &Slot, container: &mut impl Container, amount: i32) -> ItemStack {
        container.remove_item(slot.container_slot, amount)
    }

    /// Tries to remove items from the slot.
    ///
    /// Returns `Some(items)` if successful, `None` if pickup is not allowed.
    pub fn try_remove(
        slot: &Slot,
        container: &mut impl Container,
        amount: i32,
        max_amount: i32,
    ) -> Option<ItemStack> {
        if !slot.may_pickup() {
            return None;
        }

        let current = get_item(slot, container);
        if current.is_empty() {
            return None;
        }

        let to_take = amount.min(max_amount).min(current.count());
        if to_take <= 0 {
            return None;
        }

        let result = remove(slot, container, to_take);
        if result.is_empty() {
            return None;
        }

        // If slot is now empty, ensure it's properly cleared
        if get_item(slot, container).is_empty() {
            set_item(slot, container, ItemStack::empty());
        }

        Some(result)
    }

    /// Safely takes items from the slot with bounds checking.
    pub fn safe_take(
        slot: &Slot,
        container: &mut impl Container,
        amount: i32,
        max_amount: i32,
    ) -> ItemStack {
        try_remove(slot, container, amount, max_amount).unwrap_or_else(ItemStack::empty)
    }

    /// Safely inserts items into the slot.
    ///
    /// Returns the remaining items that couldn't be inserted.
    pub fn safe_insert(
        slot: &Slot,
        container: &mut impl Container,
        mut input: ItemStack,
        max_amount: i32,
    ) -> ItemStack {
        if input.is_empty() || !slot.may_place(&input) {
            return input;
        }

        let current = get_item(slot, container);
        let space_available = slot.max_stack_size_for(&input) - current.count();
        let transfer_count = input.count().min(max_amount).min(space_available);

        if transfer_count <= 0 {
            return input;
        }

        if current.is_empty() {
            let to_insert = input.split(transfer_count);
            set_item(slot, container, to_insert);
        } else if ItemStack::is_same_item_same_components(current, &input) {
            input.shrink(transfer_count);
            let new_item = get_item_mut(slot, container);
            new_item.grow(transfer_count);
            container.set_changed();
        }

        input
    }
}
