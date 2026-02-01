//! This module contains entity-related traits and types.

use std::sync::{Arc, Weak};

use steel_registry::blocks::shapes::AABBd;
use steel_registry::entity_data::DataValue;
use steel_registry::entity_types::EntityTypeRef;
use steel_registry::item_stack::ItemStack;
use steel_utils::math::Vector3;
use uuid::Uuid;

use crate::{inventory::equipment::EquipmentSlot, player::Player};

mod cache;
mod callback;
pub mod entities;
mod registry;
mod storage;
mod tracker;

pub use cache::EntityCache;
pub use callback::{
    EntityChunkCallback, EntityLevelCallback, NullEntityCallback, PlayerEntityCallback,
    RemovalReason,
};
pub use registry::{ENTITIES, EntityRegistry, init_entities};
pub use storage::EntityStorage;
pub use tracker::EntityTracker;

/// Type alias for a shared entity reference.
pub type SharedEntity = Arc<dyn Entity>;

/// Type alias for a weak entity reference.
pub type WeakEntity = Weak<dyn Entity>;

/// A trait for entities.
///
/// This trait provides the core functionality for entities.
/// It's based on Minecraft's `Entity` class.
pub trait Entity: Send + Sync {
    /// Gets the entity type containing tracking range, dimensions, etc.
    fn entity_type(&self) -> EntityTypeRef;

    /// Gets the entity's unique network ID (session-local).
    fn id(&self) -> i32;

    /// Gets the UUID of the entity (persistent identifier).
    fn uuid(&self) -> Uuid;

    /// Gets the entity's current position.
    fn position(&self) -> Vector3<f64>;

    /// Gets the entity's bounding box for collision queries.
    fn bounding_box(&self) -> AABBd;

    /// Called every game tick when the entity is in a ticked chunk.
    ///
    /// Override this to add entity-specific tick behavior.
    /// The caller (`EntityStorage`) handles base tick logic like dirty data sync.
    fn tick(&self) {}

    /// Packs dirty entity data for network synchronization.
    ///
    /// Returns `Some(values)` if there are dirty values to sync, `None` otherwise.
    /// Clears the dirty flags after packing.
    fn pack_dirty_entity_data(&self) -> Option<Vec<DataValue>> {
        None
    }

    /// Packs all non-default entity data for initial spawn.
    ///
    /// Used when sending entity data to a player who just started tracking this entity.
    fn pack_all_entity_data(&self) -> Vec<DataValue> {
        Vec::new()
    }

    /// Returns true if the entity has been marked for removal.
    fn is_removed(&self) -> bool;

    /// Marks the entity as removed with the given reason.
    fn set_removed(&self, reason: RemovalReason);

    /// Sets the level callback for lifecycle events (movement, removal).
    fn set_level_callback(&self, callback: Arc<dyn EntityLevelCallback>);

    /// Gets the entity as a Player if it is one.
    fn as_player(self: Arc<Self>) -> Option<Arc<Player>> {
        None
    }
}

/// A trait for living entities that can take damage, heal, and die.
///
/// This trait provides the core functionality for entities that have health,
/// can be damaged, and can die. It's based on Minecraft's `LivingEntity` class.
pub trait LivingEntity: Entity {
    /// Gets the current health of the entity.
    fn get_health(&self) -> f32;

    /// Sets the health of the entity, clamped between 0 and max health.
    fn set_health(&mut self, health: f32);

    /// Gets the maximum health of the entity.
    fn get_max_health(&self) -> f32;

    /// Heals the entity by the specified amount.
    fn heal(&mut self, amount: f32) {
        let current_health = self.get_health();
        if current_health > 0.0 {
            self.set_health(current_health + amount);
        }
    }

    /// Returns true if the entity is dead or dying (health <= 0).
    fn is_dead_or_dying(&self) -> bool {
        self.get_health() <= 0.0
    }

    /// Returns true if the entity is alive (health > 0).
    fn is_alive(&self) -> bool {
        !self.is_dead_or_dying()
    }

    /// Gets the entity's position.
    fn get_position(&self) -> Vector3<f64>;

    /// Gets the absorption amount (extra health from effects like absorption).
    fn get_absorption_amount(&self) -> f32;

    /// Sets the absorption amount.
    fn set_absorption_amount(&mut self, amount: f32);

    /// Gets the entity's armor value.
    fn get_armor_value(&self) -> i32;

    /// Checks if the entity can be affected by potions.
    fn is_affected_by_potions(&self) -> bool {
        true
    }

    /// Checks if the entity is attackable.
    fn attackable(&self) -> bool {
        true
    }

    /// Checks if the entity is currently using an item.
    fn is_using_item(&self) -> bool {
        false
    }

    /// Checks if the entity is blocking with a shield or similar item.
    fn is_blocking(&self) -> bool {
        false
    }

    /// Checks if the entity is fall flying (using elytra).
    fn is_fall_flying(&self) -> bool {
        false
    }

    /// Checks if the entity is sleeping.
    fn is_sleeping(&self) -> bool {
        false
    }

    /// Stops the entity from sleeping.
    fn stop_sleeping(&mut self) {}

    /// Checks if the entity is sprinting.
    fn is_sprinting(&self) -> bool {
        false
    }

    /// Sets whether the entity is sprinting.
    fn set_sprinting(&mut self, sprinting: bool);

    /// Gets the entity's speed attribute value.
    fn get_speed(&self) -> f32;

    /// Sets the entity's speed.
    fn set_speed(&mut self, speed: f32);

    // Equipment methods

    /// Gets a clone of the item in the specified equipment slot.
    ///
    /// Default implementation returns an empty stack.
    fn get_item_by_slot(&self, _slot: EquipmentSlot) -> ItemStack {
        ItemStack::empty()
    }

    /// Gets the main hand item.
    fn get_main_hand_item(&self) -> ItemStack {
        self.get_item_by_slot(EquipmentSlot::MainHand)
    }

    /// Gets the off hand item.
    fn get_off_hand_item(&self) -> ItemStack {
        self.get_item_by_slot(EquipmentSlot::OffHand)
    }

    /// Checks if the main hand slot is empty.
    fn is_main_hand_empty(&self) -> bool {
        self.get_item_by_slot(EquipmentSlot::MainHand).is_empty()
    }

    /// Checks if the off hand slot is empty.
    fn is_off_hand_empty(&self) -> bool {
        self.get_item_by_slot(EquipmentSlot::OffHand).is_empty()
    }
}
