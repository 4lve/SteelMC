//! Player inventory menu implementation.
//!
//! This is the menu that is always associated with the player (container ID 0).
//! It includes the crafting grid, armor slots, main inventory, hotbar, and offhand.

use steel_registry::item_stack::ItemStack;

use super::abstract_menu::{AbstractContainerMenu, SLOT_SIZE};
use crate::inventory::{Container, PlayerInventory, Slot};

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

/// A wrapper container that provides the 46-slot view for the inventory menu.
///
/// This maps the menu slot indices to the actual player inventory slots:
/// - Slots 0-4: Crafting (not backed by player inventory, stored here)
/// - Slots 5-8: Armor (stored here for now, TODO: integrate with equipment)
/// - Slots 9-35: Main inventory (maps to player inventory slots 9-35)
/// - Slots 36-44: Hotbar (maps to player inventory slots 0-8)
/// - Slot 45: Offhand (stored here for now, TODO: integrate with equipment)
#[derive(Debug)]
pub struct InventoryMenuContainer {
    /// Crafting result + grid (5 slots).
    crafting: [ItemStack; 5],
    /// Armor slots (4 slots: head, chest, legs, feet).
    armor: [ItemStack; 4],
    /// Offhand slot.
    offhand: ItemStack,
    /// Reference to the underlying player inventory for slots 9-44.
    /// This is a copy that should be synced back.
    main_inventory: [ItemStack; 36],
    /// Changed flag.
    changed: bool,
}

impl Default for InventoryMenuContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryMenuContainer {
    /// Creates a new inventory menu container.
    #[must_use]
    pub fn new() -> Self {
        Self {
            crafting: std::array::from_fn(|_| ItemStack::empty()),
            armor: std::array::from_fn(|_| ItemStack::empty()),
            offhand: ItemStack::empty(),
            main_inventory: std::array::from_fn(|_| ItemStack::empty()),
            changed: false,
        }
    }

    /// Copies items from a player inventory into this container.
    pub fn load_from_player_inventory(&mut self, inventory: &PlayerInventory) {
        // Copy main inventory (slots 9-35 in menu = slots 9-35 in player inv)
        for i in 0..27 {
            self.main_inventory[i] = inventory.get_item(i + 9).clone();
        }
        // Copy hotbar (slots 36-44 in menu = slots 0-8 in player inv)
        for i in 0..9 {
            self.main_inventory[27 + i] = inventory.get_item(i).clone();
        }
    }

    /// Saves items back to a player inventory.
    pub fn save_to_player_inventory(&self, inventory: &mut PlayerInventory) {
        // Save main inventory
        for i in 0..27 {
            inventory.set_item(i + 9, self.main_inventory[i].clone());
        }
        // Save hotbar
        for i in 0..9 {
            inventory.set_item(i, self.main_inventory[27 + i].clone());
        }
    }

}

impl Container for InventoryMenuContainer {
    fn size(&self) -> usize {
        slots::TOTAL_SLOTS
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        match slot {
            0..=4 => &self.crafting[slot],
            5..=8 => &self.armor[slot - 5],
            9..=44 => &self.main_inventory[slot - 9],
            45 => &self.offhand,
            // Return first crafting slot as fallback (will be empty)
            _ => &self.crafting[0],
        }
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        match slot {
            0..=4 => &mut self.crafting[slot],
            5..=8 => &mut self.armor[slot - 5],
            9..=44 => &mut self.main_inventory[slot - 9],
            45 => &mut self.offhand,
            // Return first crafting slot as fallback (will be empty)
            _ => &mut self.crafting[0],
        }
    }

    fn set_item(&mut self, slot: usize, item: ItemStack) {
        match slot {
            0..=4 => self.crafting[slot] = item,
            5..=8 => self.armor[slot - 5] = item,
            9..=44 => self.main_inventory[slot - 9] = item,
            45 => self.offhand = item,
            _ => {}
        }
        self.set_changed();
    }

    fn set_changed(&mut self) {
        self.changed = true;
    }
}

/// The player's inventory menu (always container ID 0).
pub struct InventoryMenu {
    /// The underlying abstract menu.
    pub menu: AbstractContainerMenu<InventoryMenuContainer>,
}

impl InventoryMenu {
    /// Creates a new inventory menu.
    #[must_use]
    pub fn new() -> Self {
        let container = InventoryMenuContainer::new();
        let mut menu = AbstractContainerMenu::new(None, INVENTORY_MENU_CONTAINER_ID, container);

        // Add crafting result slot (slot 0)
        menu.add_slot(Slot::new(0, 154, 28));

        // Add crafting grid slots (slots 1-4, 2x2)
        for row in 0..2 {
            for col in 0..2 {
                let slot_idx = 1 + col + row * 2;
                menu.add_slot(Slot::new(slot_idx, 98 + col as i32 * SLOT_SIZE, 18 + row as i32 * SLOT_SIZE));
            }
        }

        // Add armor slots (slots 5-8: head, chest, legs, feet)
        for i in 0..4 {
            menu.add_slot(Slot::new(5 + i, 8, 8 + i as i32 * SLOT_SIZE));
        }

        // Add main inventory slots (slots 9-35)
        for row in 0..3 {
            for col in 0..9 {
                let slot_idx = 9 + col + row * 9;
                menu.add_slot(Slot::new(slot_idx, 8 + col as i32 * SLOT_SIZE, 84 + row as i32 * SLOT_SIZE));
            }
        }

        // Add hotbar slots (slots 36-44)
        for col in 0..9 {
            let slot_idx = 36 + col;
            menu.add_slot(Slot::new(slot_idx, 8 + col as i32 * SLOT_SIZE, 142));
        }

        // Add offhand slot (slot 45)
        menu.add_slot(Slot::new(45, 77, 62));

        Self { menu }
    }

    /// Loads items from a player inventory.
    pub fn load_from(&mut self, inventory: &PlayerInventory) {
        self.menu.container.load_from_player_inventory(inventory);
    }

    /// Saves items back to a player inventory.
    pub fn save_to(&self, inventory: &mut PlayerInventory) {
        self.menu.container.save_to_player_inventory(inventory);
    }

    /// Sets an item directly in a slot (for creative mode).
    pub fn set_slot(&mut self, slot: usize, item: ItemStack) {
        if slot < self.menu.slots.len() {
            self.menu.container.set_item(slot, item);
        }
    }

    /// Gets an item from a slot.
    #[must_use]
    pub fn get_slot(&self, slot: usize) -> &ItemStack {
        self.menu.container.get_item(slot)
    }

    /// Returns the container for the menu.
    #[must_use]
    pub fn container(&self) -> &InventoryMenuContainer {
        &self.menu.container
    }

    /// Returns the container for the menu (mutable).
    #[must_use]
    pub fn container_mut(&mut self) -> &mut InventoryMenuContainer {
        &mut self.menu.container
    }
}

impl Default for InventoryMenu {
    fn default() -> Self {
        Self::new()
    }
}

