//! This module contains the inventory system.

mod container;
pub mod menu;
mod player_inventory;
mod simple_container;
mod slot;

pub use container::Container;
pub use menu::{
    AbstractContainerMenu, ChestMenu, ClickAction, ClickType, INVENTORY_MENU_CONTAINER_ID,
    InventoryMenu, MenuType, QuickCraftPhase, QuickCraftType, SLOT_SIZE, SLOTS_PER_ROW, inv_slots,
    menu_slot_to_inventory_slot,
};
pub use player_inventory::{HOTBAR_SIZE, INVENTORY_SIZE, PlayerInventory, SLOT_OFFHAND};
pub use simple_container::SimpleContainer;
pub use slot::{Slot, slot_ops};
