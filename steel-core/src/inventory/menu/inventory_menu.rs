//! Player inventory menu implementation.
//!
//! This is the menu that is always associated with the player (container ID 0).
//! It includes the crafting grid, armor slots, main inventory, hotbar, and offhand.
//!
//! Unlike other menus, this menu's slots directly reference the player's
//! `PlayerInventory` rather than copying items.

use std::sync::Arc;

use steel_registry::item_stack::ItemStack;
use steel_utils::locks::SyncMutex;

use crate::inventory::{Container, PlayerInventory, Slot};

use super::abstract_menu::SLOT_SIZE;

/// The container ID for the player's inventory menu.
pub const INVENTORY_MENU_CONTAINER_ID: i32 = 0;

/// Slot indices in the inventory menu.
pub mod slots {
    /// Crafting result slot (read-only output).
    pub const RESULT_SLOT: usize = 0;
    /// Start of crafting grid slots (2x2).
    pub const CRAFT_SLOT_START: usize = 1;
    /// End of crafting grid slots (exclusive).
    pub const CRAFT_SLOT_END: usize = 5;
    /// Start of armor slots.
    pub const ARMOR_SLOT_START: usize = 5;
    /// End of armor slots (exclusive).
    pub const ARMOR_SLOT_END: usize = 9;
    /// Start of main inventory slots.
    pub const INV_SLOT_START: usize = 9;
    /// End of main inventory slots (exclusive).
    pub const INV_SLOT_END: usize = 36;
    /// Start of hotbar slots.
    pub const HOTBAR_SLOT_START: usize = 36;
    /// End of hotbar slots (exclusive).
    pub const HOTBAR_SLOT_END: usize = 45;
    /// Offhand/shield slot.
    pub const OFFHAND_SLOT: usize = 45;
    /// Total number of slots in the inventory menu.
    pub const TOTAL_SLOTS: usize = 46;
}

/// Maps menu slot index to player inventory slot index.
///
/// Menu slot mapping:
/// - 0-4: crafting (not in player inventory, returns None)
/// - 5: HEAD armor -> inventory slot 39
/// - 6: CHEST armor -> inventory slot 38
/// - 7: LEGS armor -> inventory slot 37
/// - 8: FEET armor -> inventory slot 36
/// - 9-35: main inventory -> inventory slots 9-35
/// - 36-44: hotbar -> inventory slots 0-8
/// - 45: offhand -> inventory slot 40
#[must_use]
pub const fn menu_slot_to_inventory_slot(menu_slot: usize) -> Option<usize> {
    match menu_slot {
        0..=4 => None,                   // Crafting slots are local
        5 => Some(39),                   // HEAD
        6 => Some(38),                   // CHEST
        7 => Some(37),                   // LEGS
        8 => Some(36),                   // FEET
        9..=35 => Some(menu_slot),       // Main inventory (same indices)
        36..=44 => Some(menu_slot - 36), // Hotbar: menu 36-44 -> inv 0-8
        45 => Some(40),                  // Offhand
        _ => None,
    }
}

/// The player's inventory menu (always container ID 0).
///
/// This menu references the player's inventory directly for most slots.
/// Only the crafting slots (0-4) are stored locally.
pub struct InventoryMenu {
    /// The slot definitions (coordinates, callbacks).
    pub slots: Vec<Slot>,
    /// Reference to the player's inventory.
    inventory: Arc<SyncMutex<PlayerInventory>>,
    /// Crafting result + grid (5 slots). Stored locally, not persisted.
    crafting: [ItemStack; 5],
    /// The item being carried on the cursor.
    pub carried: ItemStack,
    /// State ID for synchronization.
    state_id: i32,
}

impl InventoryMenu {
    /// Creates a new inventory menu with a reference to the player's inventory.
    #[must_use]
    pub fn new(inventory: Arc<SyncMutex<PlayerInventory>>) -> Self {
        let mut slots = Vec::with_capacity(slots::TOTAL_SLOTS);

        // Add crafting result slot (slot 0)
        slots.push(Slot::new(0, 154, 28));

        // Add crafting grid slots (slots 1-4, 2x2)
        for row in 0..2 {
            for col in 0..2 {
                let slot_idx = 1 + col + row * 2;
                slots.push(Slot::new(
                    slot_idx,
                    98 + col as i32 * SLOT_SIZE,
                    18 + row as i32 * SLOT_SIZE,
                ));
            }
        }

        // Add armor slots (slots 5-8: head, chest, legs, feet)
        for i in 0..4 {
            slots.push(Slot::new(5 + i, 8, 8 + i as i32 * SLOT_SIZE));
        }

        // Add main inventory slots (slots 9-35)
        for row in 0..3 {
            for col in 0..9 {
                let slot_idx = 9 + col + row * 9;
                slots.push(Slot::new(
                    slot_idx,
                    8 + col as i32 * SLOT_SIZE,
                    84 + row as i32 * SLOT_SIZE,
                ));
            }
        }

        // Add hotbar slots (slots 36-44)
        for col in 0..9 {
            let slot_idx = 36 + col;
            slots.push(Slot::new(slot_idx, 8 + col as i32 * SLOT_SIZE, 142));
        }

        // Add offhand slot (slot 45)
        slots.push(Slot::new(45, 77, 62));

        Self {
            slots,
            inventory,
            crafting: std::array::from_fn(|_| ItemStack::empty()),
            carried: ItemStack::empty(),
            state_id: 0,
        }
    }

    /// Returns the state ID.
    #[must_use]
    pub fn state_id(&self) -> i32 {
        self.state_id
    }

    /// Increments and returns the new state ID.
    pub fn next_state_id(&mut self) -> i32 {
        self.state_id = self.state_id.wrapping_add(1);
        self.state_id
    }

    /// Gets an item from a menu slot.
    #[must_use]
    pub fn get_item(&self, menu_slot: usize) -> ItemStack {
        if menu_slot < 5 {
            // Crafting slots are local
            self.crafting[menu_slot].clone()
        } else if let Some(inv_slot) = menu_slot_to_inventory_slot(menu_slot) {
            // Delegate to player inventory
            let inv = self.inventory.lock();
            inv.get_item(inv_slot).clone()
        } else {
            ItemStack::empty()
        }
    }

    /// Sets an item in a menu slot.
    pub fn set_item(&mut self, menu_slot: usize, item: ItemStack) {
        if menu_slot < 5 {
            // Crafting slots are local
            self.crafting[menu_slot] = item;
        } else if let Some(inv_slot) = menu_slot_to_inventory_slot(menu_slot) {
            // Delegate to player inventory
            let mut inv = self.inventory.lock();
            inv.set_item(inv_slot, item);
        }
    }

    /// Gets a mutable reference to a crafting slot.
    /// Returns None for non-crafting slots since those are in the inventory.
    pub fn get_crafting_slot_mut(&mut self, menu_slot: usize) -> Option<&mut ItemStack> {
        if menu_slot < 5 {
            Some(&mut self.crafting[menu_slot])
        } else {
            None
        }
    }

    /// Returns the number of slots in the menu.
    #[must_use]
    pub fn size(&self) -> usize {
        slots::TOTAL_SLOTS
    }

    /// Gets the carried item (on cursor).
    #[must_use]
    pub fn get_carried(&self) -> &ItemStack {
        &self.carried
    }

    /// Sets the carried item.
    pub fn set_carried(&mut self, item: ItemStack) {
        self.carried = item;
    }

    /// Clears crafting slots and returns items to the player.
    pub fn clear_crafting(&mut self) -> Vec<ItemStack> {
        let mut items = Vec::new();
        for slot in &mut self.crafting {
            if !slot.is_empty() {
                items.push(slot.copy_and_clear());
            }
        }
        items
    }

    /// Returns whether the menu is still valid for the player.
    #[must_use]
    pub fn still_valid(&self) -> bool {
        // Player's inventory menu is always valid
        true
    }

    /// Collects all slot contents for network sync.
    pub fn collect_all_items(&self) -> Vec<ItemStack> {
        let mut items = Vec::with_capacity(slots::TOTAL_SLOTS);
        for slot in 0..slots::TOTAL_SLOTS {
            items.push(self.get_item(slot));
        }
        items
    }

    /// Provides access to the underlying inventory for advanced operations.
    pub fn with_inventory<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&PlayerInventory) -> R,
    {
        let inv = self.inventory.lock();
        f(&inv)
    }

    /// Provides mutable access to the underlying inventory.
    pub fn with_inventory_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut PlayerInventory) -> R,
    {
        let mut inv = self.inventory.lock();
        f(&mut inv)
    }
}

/// Container trait implementation that provides a unified view.
impl Container for InventoryMenu {
    fn size(&self) -> usize {
        slots::TOTAL_SLOTS
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        // This is tricky - we can't return a reference to inventory items
        // since they're behind a mutex. For now, panic for non-crafting slots.
        // Use get_item() method instead which returns owned ItemStack.
        if slot < 5 {
            &self.crafting[slot]
        } else {
            // For inventory slots, caller should use get_item() method instead
            panic!(
                "Cannot get reference to inventory slot {}. Use InventoryMenu::get_item() instead.",
                slot
            );
        }
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        if slot < 5 {
            &mut self.crafting[slot]
        } else {
            panic!(
                "Cannot get mutable reference to inventory slot {}. Use InventoryMenu::set_item() instead.",
                slot
            );
        }
    }

    fn set_item(&mut self, slot: usize, item: ItemStack) {
        InventoryMenu::set_item(self, slot, item);
    }

    fn set_changed(&mut self) {
        // The underlying inventory handles its own change tracking
    }
}
