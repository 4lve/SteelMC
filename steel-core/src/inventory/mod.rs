//! This module contains the inventory system.

mod container;
pub mod menu;
mod player_inventory;
mod simple_container;
mod slot;

pub use container::Container;
pub use menu::{
    AbstractContainerMenu, ChestMenu, ClickAction, ClickType, MenuType, QuickCraftPhase,
    QuickCraftType, SLOT_SIZE, SLOTS_PER_ROW,
};
pub use player_inventory::{HOTBAR_SIZE, INVENTORY_SIZE, PlayerInventory, SLOT_OFFHAND};
pub use simple_container::SimpleContainer;
pub use slot::{Slot, slot_ops};
