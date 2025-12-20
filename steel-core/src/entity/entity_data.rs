//! Entity data synchronization system
//!
//! This module implements the synced entity data system that tracks changes
//! to entity properties and efficiently broadcasts only dirty (changed) values.

use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use steel_utils::locks::SyncRwLock;

use super::Pose;

/// A type-erased container for entity data values
pub struct EntityDataValue {
    type_id: TypeId,
    value: Arc<dyn Any + Send + Sync>,
    serializer_id: u8,
}

impl Clone for EntityDataValue {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            value: Arc::clone(&self.value),
            serializer_id: self.serializer_id,
        }
    }
}

impl EntityDataValue {
    /// Creates a new entity data value
    pub fn new<T: Clone + Send + Sync + 'static>(value: T, serializer_id: u8) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            value: Arc::new(value),
            serializer_id,
        }
    }

    /// Gets the value as a specific type
    #[must_use]
    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        if self.type_id == TypeId::of::<T>() {
            self.value.downcast_ref::<T>().cloned()
        } else {
            None
        }
    }

    /// Gets the serializer ID for this value type
    #[must_use]
    pub fn serializer_id(&self) -> u8 {
        self.serializer_id
    }
}

/// A data item that tracks its dirty state
struct DataItem {
    value: EntityDataValue,
    dirty: AtomicBool,
}

impl DataItem {
    fn new(value: EntityDataValue) -> Self {
        Self {
            value,
            dirty: AtomicBool::new(true), // Start dirty so initial spawn sends all data
        }
    }

    fn set_value(&mut self, value: EntityDataValue) {
        self.value = value;
        self.dirty.store(true, Ordering::Release);
    }

    fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    fn mark_clean(&self) {
        self.dirty.store(false, Ordering::Release);
    }
}

/// Entity data storage with dirty tracking
pub struct EntityData {
    entity_id: i32,
    items: SyncRwLock<FxHashMap<u8, DataItem>>,
    is_dirty: AtomicBool,
}

impl EntityData {
    /// Creates a new entity data storage
    #[must_use]
    pub fn new(entity_id: i32) -> Self {
        Self {
            entity_id,
            items: SyncRwLock::new(FxHashMap::default()),
            is_dirty: AtomicBool::new(false),
        }
    }

    /// Defines a new data field with an initial value
    pub fn define<T: Clone + Send + Sync + 'static>(
        &mut self,
        accessor: EntityDataAccessor<T>,
        initial_value: T,
    ) {
        let value = EntityDataValue::new(initial_value, accessor.serializer_id);
        let item = DataItem::new(value);
        self.items.write().insert(accessor.id, item);
    }

    /// Sets a data field value
    pub fn set<T: Clone + Send + Sync + 'static>(&self, accessor: EntityDataAccessor<T>, value: T) {
        let new_value = EntityDataValue::new(value, accessor.serializer_id);
        let mut items = self.items.write();

        if let Some(item) = items.get_mut(&accessor.id) {
            item.set_value(new_value);
            self.is_dirty.store(true, Ordering::Release);
        } else {
            // If not defined, define it now
            let item = DataItem::new(new_value);
            items.insert(accessor.id, item);
            self.is_dirty.store(true, Ordering::Release);
        }
    }

    /// Gets a data field value
    ///
    /// # Panics
    ///
    /// Panics if the field is not defined or if there is a type mismatch.
    pub fn get<T: Clone + 'static>(&self, accessor: EntityDataAccessor<T>) -> T {
        let items = self.items.read();
        items
            .get(&accessor.id)
            .and_then(|item| item.value.get::<T>())
            .expect("Entity data field not defined or type mismatch")
    }

    /// Checks if any data has been modified
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::Acquire)
    }

    /// Packs all dirty data values into a vec, marking them as clean
    pub fn pack_dirty(&self) -> Option<Vec<(u8, EntityDataValue)>> {
        if !self.is_dirty() {
            return None;
        }

        let items = self.items.read();
        let dirty_items: Vec<(u8, EntityDataValue)> = items
            .iter()
            .filter(|(_, item)| item.is_dirty())
            .map(|(id, item)| {
                item.mark_clean();
                (*id, item.value.clone())
            })
            .collect();

        if dirty_items.is_empty() {
            self.is_dirty.store(false, Ordering::Release);
            None
        } else {
            self.is_dirty.store(false, Ordering::Release);
            Some(dirty_items)
        }
    }

    /// Packs all data values (used for initial entity spawn)
    pub fn pack_all(&self) -> Vec<(u8, EntityDataValue)> {
        let items = self.items.read();
        items
            .iter()
            .map(|(id, item)| {
                item.mark_clean();
                (*id, item.value.clone())
            })
            .collect()
    }

    /// Gets the entity ID this data belongs to
    pub fn entity_id(&self) -> i32 {
        self.entity_id
    }
}

/// Type-safe accessor for entity data fields
pub struct EntityDataAccessor<T> {
    id: u8,
    serializer_id: u8,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> EntityDataAccessor<T> {
    /// Creates a new data accessor
    #[must_use]
    pub const fn new(id: u8, serializer_id: u8) -> Self {
        Self {
            id,
            serializer_id,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Gets the data field ID
    #[must_use]
    pub fn id(&self) -> u8 {
        self.id
    }
}

impl<T> Clone for EntityDataAccessor<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EntityDataAccessor<T> {}

/// Standard entity data accessors
impl EntityDataAccessor<u8> {
    /// Shared flags (fire, crouch, sprint, etc.)
    pub const SHARED_FLAGS: Self = Self::new(0, EntityDataSerializers::BYTE);
    /// Player model parts
    pub const PLAYER_MODEL_PARTS: Self = Self::new(18, EntityDataSerializers::BYTE);
}

impl EntityDataAccessor<i32> {
    /// Air supply (for drowning)
    pub const AIR_SUPPLY: Self = Self::new(1, EntityDataSerializers::INT);

    /// Frozen ticks (for powder snow)
    pub const FROZEN_TICKS: Self = Self::new(7, EntityDataSerializers::INT);
}

impl EntityDataAccessor<Option<String>> {
    /// Custom name (optional text component)
    pub const CUSTOM_NAME: Self = Self::new(2, EntityDataSerializers::OPTIONAL_TEXT_COMPONENT);
}

impl EntityDataAccessor<bool> {
    /// Whether custom name is visible
    pub const CUSTOM_NAME_VISIBLE: Self = Self::new(3, EntityDataSerializers::BOOLEAN);

    /// Whether entity is silent
    pub const SILENT: Self = Self::new(4, EntityDataSerializers::BOOLEAN);

    /// Whether entity has no gravity
    pub const NO_GRAVITY: Self = Self::new(5, EntityDataSerializers::BOOLEAN);
}

impl EntityDataAccessor<Pose> {
    /// Entity pose (standing, crouching, etc.)
    pub const POSE: Self = Self::new(6, EntityDataSerializers::POSE);
}

/// Serializer IDs for different data types
pub struct EntityDataSerializers;

impl EntityDataSerializers {
    /// Byte serializer (u8)
    pub const BYTE: u8 = 0;
    /// Integer serializer (i32/VarInt)
    pub const INT: u8 = 1;
    /// Long serializer (i64)
    pub const LONG: u8 = 2;
    /// Float serializer (f32)
    pub const FLOAT: u8 = 3;
    /// String serializer
    pub const STRING: u8 = 4;
    /// Text component serializer
    pub const TEXT_COMPONENT: u8 = 5;
    /// Optional text component serializer
    pub const OPTIONAL_TEXT_COMPONENT: u8 = 6;
    /// Item stack serializer
    pub const ITEM_STACK: u8 = 7;
    /// Boolean serializer
    pub const BOOLEAN: u8 = 8;
    /// Rotations serializer
    pub const ROTATIONS: u8 = 9;
    /// Block position serializer
    pub const BLOCK_POS: u8 = 10;
    /// Optional block position serializer
    pub const OPTIONAL_BLOCK_POS: u8 = 11;
    /// Direction serializer
    pub const DIRECTION: u8 = 12;
    /// Optional UUID serializer
    pub const OPTIONAL_UUID: u8 = 13;
    /// Block state serializer
    pub const BLOCK_STATE: u8 = 14;
    /// Optional block state serializer
    pub const OPTIONAL_BLOCK_STATE: u8 = 15;
    /// NBT serializer
    pub const NBT: u8 = 16;
    /// Particle serializer
    pub const PARTICLE: u8 = 17;
    /// Villager data serializer
    pub const VILLAGER_DATA: u8 = 18;
    /// Optional integer serializer
    pub const OPTIONAL_INT: u8 = 19;
    /// Pose serializer
    pub const POSE: u8 = 20;
    /// Cat variant serializer
    pub const CAT_VARIANT: u8 = 21;
    /// Frog variant serializer
    pub const FROG_VARIANT: u8 = 22;
    /// Optional global position serializer
    pub const OPTIONAL_GLOBAL_POS: u8 = 23;
    /// Painting variant serializer
    pub const PAINTING_VARIANT: u8 = 24;
    /// Sniffer state serializer
    pub const SNIFFER_STATE: u8 = 25;
    /// Vector3 serializer
    pub const VECTOR3: u8 = 26;
    /// Quaternion serializer
    pub const QUATERNION: u8 = 27;
    /// Optional string serializer
    pub const OPTIONAL_STRING: u8 = 28;
}
