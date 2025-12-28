//! Entity equipment holder.
//!
//! This module provides the `EntityEquipment` struct which holds all equipment
//! items for an entity (armor, hands, saddle, etc.).

use std::collections::HashMap;

use steel_registry::item_stack::ItemStack;

use super::EquipmentSlot;

/// Holds equipment items for an entity.
///
/// This is a separate holder that entities own. The player's `Inventory` references
/// this to provide a unified view of all items (main inventory + equipment).
#[derive(Debug, Clone)]
pub struct EntityEquipment {
    /// Items stored by equipment slot.
    items: HashMap<EquipmentSlot, ItemStack>,
}

impl Default for EntityEquipment {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityEquipment {
    /// Creates a new empty equipment holder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Sets the item in a slot, returning the previous item.
    pub fn set(&mut self, slot: EquipmentSlot, item: ItemStack) -> ItemStack {
        self.items
            .insert(slot, item)
            .unwrap_or_else(ItemStack::empty)
    }

    /// Gets the item in a slot (cloned).
    #[must_use]
    pub fn get(&self, slot: EquipmentSlot) -> ItemStack {
        self.items
            .get(&slot)
            .cloned()
            .unwrap_or_else(ItemStack::empty)
    }

    /// Gets a reference to the item in a slot, if present.
    #[must_use]
    pub fn get_ref(&self, slot: EquipmentSlot) -> Option<&ItemStack> {
        self.items.get(&slot)
    }

    /// Gets a mutable reference to the item in a slot.
    ///
    /// Note: This creates an empty item in the slot if none exists.
    pub fn get_mut(&mut self, slot: EquipmentSlot) -> &mut ItemStack {
        self.items.entry(slot).or_insert_with(ItemStack::empty)
    }

    /// Returns whether all equipment slots are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.values().all(ItemStack::is_empty)
    }

    /// Clears all equipment slots.
    pub fn clear(&mut self) {
        for item in self.items.values_mut() {
            *item = ItemStack::empty();
        }
    }

    /// Copies all equipment from another holder.
    pub fn set_all(&mut self, other: &EntityEquipment) {
        self.items.clear();
        for (slot, item) in &other.items {
            self.items.insert(*slot, item.clone());
        }
    }

    /// Returns an iterator over all non-empty equipment slots.
    pub fn iter(&self) -> impl Iterator<Item = (EquipmentSlot, &ItemStack)> {
        self.items
            .iter()
            .filter(|(_, item)| !item.is_empty())
            .map(|(slot, item)| (*slot, item))
    }

    /// Returns an iterator over all equipment slots (including empty).
    pub fn iter_all(&self) -> impl Iterator<Item = (EquipmentSlot, ItemStack)> + '_ {
        EquipmentSlot::VALUES
            .iter()
            .map(|slot| (*slot, self.get(*slot)))
    }

    // TODO: Add tick() method when Entity trait exists
    // TODO: Add dropAll() method when Entity trait exists
}
