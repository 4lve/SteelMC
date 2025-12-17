//! Entity system for `SteelMC`
//!
//! This module contains the entity tracking and synchronization systems
//! that allow players to see each other and their actions.

pub mod entity_data;
pub mod entity_tracker;
pub mod packet_helpers;
pub mod player_entity;
pub mod tracked_entity;

pub use entity_data::{EntityData, EntityDataAccessor, EntityDataSerializers, EntityDataValue};
pub use entity_tracker::EntityTracker;
pub use packet_helpers::{entity_data_to_packet_entries, serialize_entity_data_value};
pub use player_entity::PlayerEntity;
pub use tracked_entity::TrackedEntity;

use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};
use steel_utils::locks::SyncMutex;
use steel_utils::math::Vector3;
use uuid::Uuid;

/// Core entity trait that all entities must implement
pub trait Entity: Send + Sync {
    /// Get the entity's unique ID
    fn entity_id(&self) -> i32;

    /// Get the entity's UUID
    fn uuid(&self) -> Uuid;

    /// Get the entity's position
    fn position(&self) -> Vector3<f64>;

    /// Get the entity's rotation (yaw, pitch)
    fn rotation(&self) -> (f32, f32);

    /// Get the entity's velocity/delta movement
    fn delta_movement(&self) -> Vector3<f64>;

    /// Get the entity's synchronized data
    fn entity_data(&self) -> &EntityData;

    /// Called when the entity becomes visible to a player
    fn start_seen_by_player(&self, _player_uuid: Uuid) {}

    /// Called when the entity is no longer visible to a player
    fn remove_seen_by_player(&self, _player_uuid: Uuid) {}
}

/// Represents an entity's pose (standing, crouching, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Pose {
    /// Standing normally
    #[default]
    Standing = 0,
    /// Flying with elytra
    FallFlying = 1,
    /// Sleeping in a bed
    Sleeping = 2,
    /// Swimming in water
    Swimming = 3,
    /// Performing spin attack
    SpinAttack = 4,
    /// Crouching/sneaking
    Crouching = 5,
    /// Long jumping (goat)
    LongJumping = 6,
    /// Dying animation
    Dying = 7,
    /// Croaking (frog)
    Croaking = 8,
    /// Using tongue (frog)
    UsingTongue = 9,
    /// Sitting (cat/wolf)
    Sitting = 10,
    /// Roaring (warden)
    Roaring = 11,
    /// Sniffing (sniffer)
    Sniffing = 12,
    /// Emerging (warden)
    Emerging = 13,
    /// Digging (sniffer/warden)
    Digging = 14,
    /// Sliding (powder snow)
    Sliding = 15,
    /// Shooting (crossbow)
    Shooting = 16,
    /// Inhaling (breath attack)
    Inhaling = 17,
}

/// Base entity implementation that can be used for players and other entities
pub struct BaseEntity {
    /// Unique entity ID (incremental)
    pub entity_id: AtomicI32,

    /// Entity UUID
    pub uuid: Uuid,

    /// Entity position
    pub position: SyncMutex<Vector3<f64>>,

    /// Entity rotation (yaw, pitch in degrees)
    pub rotation: SyncMutex<(f32, f32)>,

    /// Entity velocity/delta movement
    pub delta_movement: SyncMutex<Vector3<f64>>,

    /// Synchronized entity data
    pub entity_data: EntityData,

    /// Whether the entity is on fire (bit 0)
    /// Whether the entity is crouching/shifting (bit 1)
    /// Whether the entity is sprinting (bit 3)
    /// Whether the entity is swimming (bit 4)
    /// Whether the entity is invisible (bit 5)
    /// Whether the entity is glowing (bit 6)
    /// Whether the entity is flying with elytra (bit 7)
    shared_flags: AtomicU8,
}

impl BaseEntity {
    /// Creates a new base entity
    #[must_use]
    pub fn new(entity_id: i32, uuid: Uuid, position: Vector3<f64>) -> Self {
        let mut entity_data = EntityData::new(entity_id);

        // Register default entity data fields
        entity_data.define(EntityDataAccessor::SHARED_FLAGS, 0u8);
        entity_data.define(EntityDataAccessor::AIR_SUPPLY, 300i32);
        entity_data.define(EntityDataAccessor::CUSTOM_NAME, None);
        entity_data.define(EntityDataAccessor::CUSTOM_NAME_VISIBLE, false);
        entity_data.define(EntityDataAccessor::SILENT, false);
        entity_data.define(EntityDataAccessor::NO_GRAVITY, false);
        entity_data.define(EntityDataAccessor::POSE, Pose::Standing);
        entity_data.define(EntityDataAccessor::FROZEN_TICKS, 0i32);

        Self {
            entity_id: AtomicI32::new(entity_id),
            uuid,
            position: SyncMutex::new(position),
            rotation: SyncMutex::new((0.0, 0.0)),
            delta_movement: SyncMutex::new(Vector3::default()),
            entity_data,
            shared_flags: AtomicU8::new(0),
        }
    }

    /// Sets a shared flag bit
    pub fn set_shared_flag(&self, bit: u8, value: bool) {
        let mut flags = self.shared_flags.load(Ordering::Relaxed);
        if value {
            flags |= 1 << bit;
        } else {
            flags &= !(1 << bit);
        }
        self.shared_flags.store(flags, Ordering::Relaxed);
        self.entity_data
            .set(EntityDataAccessor::SHARED_FLAGS, flags);
    }

    /// Gets a shared flag bit
    pub fn get_shared_flag(&self, bit: u8) -> bool {
        let flags = self.shared_flags.load(Ordering::Relaxed);
        (flags & (1 << bit)) != 0
    }

    /// Sets whether the entity is on fire
    pub fn set_on_fire(&self, on_fire: bool) {
        self.set_shared_flag(0, on_fire);
    }

    /// Sets whether the entity is crouching (shift key down)
    pub fn set_shift_key_down(&self, crouching: bool) {
        self.set_shared_flag(1, crouching);
    }

    /// Gets whether the entity is crouching
    pub fn is_shift_key_down(&self) -> bool {
        self.get_shared_flag(1)
    }

    /// Sets whether the entity is sprinting
    pub fn set_sprinting(&self, sprinting: bool) {
        self.set_shared_flag(3, sprinting);
    }

    /// Gets whether the entity is sprinting
    pub fn is_sprinting(&self) -> bool {
        self.get_shared_flag(3)
    }

    /// Sets whether the entity is swimming
    pub fn set_swimming(&self, swimming: bool) {
        self.set_shared_flag(4, swimming);
    }

    /// Sets whether the entity is invisible
    pub fn set_invisible(&self, invisible: bool) {
        self.set_shared_flag(5, invisible);
    }

    /// Sets whether the entity is glowing
    pub fn set_glowing(&self, glowing: bool) {
        self.set_shared_flag(6, glowing);
    }

    /// Sets whether the entity is flying with elytra
    pub fn set_fall_flying(&self, flying: bool) {
        self.set_shared_flag(7, flying);
    }

    /// Sets the entity's pose
    pub fn set_pose(&self, pose: Pose) {
        self.entity_data.set(EntityDataAccessor::POSE, pose);
    }

    /// Gets the entity's pose
    pub fn pose(&self) -> Pose {
        self.entity_data.get(EntityDataAccessor::POSE)
    }

    /// Checks if the entity has a specific pose
    pub fn has_pose(&self, pose: Pose) -> bool {
        self.pose() == pose
    }

    /// Checks if the entity is crouching
    pub fn is_crouching(&self) -> bool {
        self.has_pose(Pose::Crouching)
    }
}

impl Entity for BaseEntity {
    fn entity_id(&self) -> i32 {
        self.entity_id.load(Ordering::Relaxed)
    }

    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn position(&self) -> Vector3<f64> {
        *self.position.lock()
    }

    fn rotation(&self) -> (f32, f32) {
        *self.rotation.lock()
    }

    fn delta_movement(&self) -> Vector3<f64> {
        *self.delta_movement.lock()
    }

    fn entity_data(&self) -> &EntityData {
        &self.entity_data
    }
}
