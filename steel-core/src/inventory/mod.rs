//! Inventory and container management system.
//!
//! This module provides the core inventory system including containers,
//! menus, crafting, equipment, and recipes.

pub mod container;
pub mod crafting;
pub mod equipment;
pub mod inventory_menu;
pub mod lock;
pub mod menu;
pub mod recipe_manager;
pub mod slot;

pub use lock::SyncPlayerInv;
