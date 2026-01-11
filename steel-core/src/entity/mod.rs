//! Entity system

pub mod entity_data;
pub mod packet_helpers;

pub use entity_data::{EntityDataAccessor, EntityDataValue};
pub use packet_helpers::{entity_data_to_packet_entries, serialize_entity_data_value};
pub use steel_registry::Pose;

use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam::atomic::AtomicCell;
use steel_registry::item_stack::ItemStack;
use steel_registry::{EntityDimensions, EntityTypeRef};
use steel_utils::math::Vector3;
use uuid::Uuid;

use crate::inventory::equipment::EquipmentSlot;

/// Common entity fields shared by all entity types (Player, mobs, etc.)
pub struct EntityBase {
    /// Unique entity ID for this server session.
    pub entity_id: i32,
    /// Persistent UUID for this entity.
    pub uuid: Uuid,
    /// Current position in the world.
    pub position: AtomicCell<Vector3<f64>>,
    /// Current rotation (yaw, pitch).
    pub rotation: AtomicCell<(f32, f32)>,
    /// Current velocity.
    pub velocity: AtomicCell<Vector3<f32>>,
    /// Current pose (standing, sneaking, swimming, etc.)
    pub pose: AtomicCell<Pose>,
    /// Whether the entity is on the ground.
    pub on_ground: AtomicBool,
    /// Shared entity flags (on fire, sneaking, sprinting, etc.)
    shared_flags: AtomicCell<u8>,
    /// Last synced pose for dirty detection.
    last_synced_pose: AtomicCell<Pose>,
    /// Last synced flags for dirty detection.
    last_synced_flags: AtomicCell<u8>,
}

impl EntityBase {
    /// Creates a new entity base with the given ID and UUID.
    #[must_use]
    pub fn new(entity_id: i32, uuid: Uuid) -> Self {
        Self {
            entity_id,
            uuid,
            position: AtomicCell::new(Vector3::default()),
            rotation: AtomicCell::new((0.0, 0.0)),
            velocity: AtomicCell::new(Vector3::default()),
            pose: AtomicCell::new(Pose::Standing),
            on_ground: AtomicBool::new(false),
            shared_flags: AtomicCell::new(0),
            last_synced_pose: AtomicCell::new(Pose::Standing),
            last_synced_flags: AtomicCell::new(0),
        }
    }

    /// Gets the shared flags byte.
    #[must_use]
    pub fn shared_flags(&self) -> u8 {
        self.shared_flags.load()
    }

    /// Sets a flag bit atomically.
    pub fn set_flag(&self, bit: u8, value: bool) {
        let mask = 1u8 << bit;
        loop {
            let old = self.shared_flags.load();
            let new = if value { old | mask } else { old & !mask };
            if self.shared_flags.compare_exchange(old, new).is_ok() {
                break;
            }
        }
    }

    /// Gets a flag bit.
    #[must_use]
    pub fn get_flag(&self, bit: u8) -> bool {
        (self.shared_flags.load() & (1 << bit)) != 0
    }

    /// Packs base entity data for initial spawn.
    #[must_use]
    pub fn pack_entity_data(&self) -> Vec<(u8, EntityDataValue)> {
        let pose = self.pose.load();
        let flags = self.shared_flags.load();
        self.last_synced_pose.store(pose);
        self.last_synced_flags.store(flags);

        vec![
            (
                EntityDataAccessor::<u8>::SHARED_FLAGS.id(),
                EntityDataValue::Byte(flags),
            ),
            (
                EntityDataAccessor::<Pose>::POSE.id(),
                EntityDataValue::Pose(pose),
            ),
        ]
    }

    /// Packs dirty base entity data. Returns None if nothing changed.
    #[must_use]
    pub fn pack_dirty_entity_data(&self) -> Option<Vec<(u8, EntityDataValue)>> {
        let mut dirty = Vec::new();

        let current_pose = self.pose.load();
        let last_pose = self.last_synced_pose.load();
        if current_pose != last_pose {
            self.last_synced_pose.store(current_pose);
            dirty.push((
                EntityDataAccessor::<Pose>::POSE.id(),
                EntityDataValue::Pose(current_pose),
            ));
        }

        let current_flags = self.shared_flags.load();
        let last_flags = self.last_synced_flags.load();
        if current_flags != last_flags {
            self.last_synced_flags.store(current_flags);
            dirty.push((
                EntityDataAccessor::<u8>::SHARED_FLAGS.id(),
                EntityDataValue::Byte(current_flags),
            ));
        }

        if dirty.is_empty() { None } else { Some(dirty) }
    }
}

// Re-export generated entity flags
pub use steel_registry::vanilla_entity_flags::*;

/// Core trait for all entities.
pub trait Entity {
    /// Returns a reference to the entity's base data.
    fn base(&self) -> &EntityBase;

    /// Returns the entity type.
    fn entity_type(&self) -> EntityTypeRef;

    /// Returns the dimensions for the current pose.
    fn dimensions(&self) -> EntityDimensions;

    /// Returns the entity ID.
    fn entity_id(&self) -> i32 {
        self.base().entity_id
    }

    /// Returns the entity's UUID.
    fn uuid(&self) -> Uuid {
        self.base().uuid
    }

    /// Returns the entity's position.
    fn position(&self) -> Vector3<f64> {
        self.base().position.load()
    }

    /// Sets the entity's position.
    fn set_position(&self, pos: Vector3<f64>) {
        self.base().position.store(pos);
    }

    /// Returns the entity's rotation (yaw, pitch).
    fn rotation(&self) -> (f32, f32) {
        self.base().rotation.load()
    }

    /// Sets the entity's rotation.
    fn set_rotation(&self, yaw: f32, pitch: f32) {
        self.base().rotation.store((yaw, pitch));
    }

    /// Returns the entity's current pose.
    fn pose(&self) -> Pose {
        self.base().pose.load()
    }

    /// Sets the entity's pose.
    fn set_pose(&self, pose: Pose) {
        self.base().pose.store(pose);
    }

    /// Returns the entity's eye height based on current pose.
    fn eye_height(&self) -> f32 {
        self.dimensions().eye_height
    }

    /// Returns whether the entity is on the ground.
    fn on_ground(&self) -> bool {
        self.base().on_ground.load(Ordering::Relaxed)
    }

    /// Packs all entity data for initial spawn.
    fn pack_entity_data(&self) -> Vec<(u8, EntityDataValue)> {
        self.base().pack_entity_data()
    }

    /// Packs dirty entity data for incremental sync.
    fn pack_dirty_entity_data(&self) -> Option<Vec<(u8, EntityDataValue)>> {
        self.base().pack_dirty_entity_data()
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
        self.base().get_flag(FLAG_SPRINTING)
    }

    /// Sets whether the entity is sprinting.
    fn set_sprinting(&self, sprinting: bool) {
        self.base().set_flag(FLAG_SPRINTING, sprinting);
    }

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
