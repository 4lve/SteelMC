//! Entity system.
//!
//! This module contains entity-related types and systems.

mod equipment;
mod equipment_slot;

pub use equipment::EntityEquipment;
pub use equipment_slot::{EquipmentSlot, EquipmentSlotType};
