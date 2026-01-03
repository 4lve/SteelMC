//! Entity data synchronization system
//!
//! This module implements the synced entity data system that tracks changes
//! to entity properties and efficiently broadcasts only dirty (changed) values.

use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use steel_utils::locks::SyncRwLock;

use super::Pose;

/// Entity data value - a strongly typed enum for all possible entity metadata values
#[derive(Debug, Clone, PartialEq)]
pub enum EntityDataValue {
    /// Byte value (u8)
    Byte(u8),
    /// Integer value (i32, sent as `VarInt`)
    Int(i32),
    /// Long value (i64)
    Long(i64),
    /// Float value (f32)
    Float(f32),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Entity pose
    Pose(Pose),
    /// Optional string (for custom names etc)
    OptionalString(Option<String>),
    /// Optional text component (for custom names with formatting)
    OptionalTextComponent(Option<String>),
    // TODO: Add more variants as needed:
    // TextComponent(TextComponent),
    // ItemStack(ItemStack),
    // Rotations(f32, f32, f32),
    // BlockPos(BlockPos),
    // OptionalBlockPos(Option<BlockPos>),
    // Direction(Direction),
    // OptionalUuid(Option<Uuid>),
    // BlockState(BlockState),
    // OptionalBlockState(Option<BlockState>),
    // CompoundTag(NbtCompound),
    // Particle(Particle),
    // Particles(Vec<Particle>),
    // VillagerData(VillagerData),
    // OptionalInt(Option<i32>),
    // CatVariant(i32),
    // WolfVariant(i32),
    // FrogVariant(i32),
    // OptionalGlobalPos(Option<GlobalPos>),
    // PaintingVariant(i32),
    // SnifferState(i32),
    // ArmadilloState(i32),
    // Vector3(f32, f32, f32),
    // Quaternion(f32, f32, f32, f32),
}

impl EntityDataValue {
    /// Gets the serializer ID for this value type
    #[must_use]
    pub fn serializer_id(&self) -> u8 {
        match self {
            Self::Byte(_) => EntityDataSerializers::BYTE,
            Self::Int(_) => EntityDataSerializers::INT,
            Self::Long(_) => EntityDataSerializers::LONG,
            Self::Float(_) => EntityDataSerializers::FLOAT,
            Self::String(_) => EntityDataSerializers::STRING,
            Self::Boolean(_) => EntityDataSerializers::BOOLEAN,
            Self::Pose(_) => EntityDataSerializers::POSE,
            Self::OptionalString(_) => EntityDataSerializers::OPTIONAL_STRING,
            Self::OptionalTextComponent(_) => EntityDataSerializers::OPTIONAL_TEXT_COMPONENT,
        }
    }

    /// Gets the value as a byte, if it is one
    #[must_use]
    pub fn as_byte(&self) -> Option<u8> {
        match self {
            Self::Byte(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as an int, if it is one
    #[must_use]
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as a long, if it is one
    #[must_use]
    pub fn as_long(&self) -> Option<i64> {
        match self {
            Self::Long(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as a float, if it is one
    #[must_use]
    pub fn as_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as a string, if it is one
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the value as a boolean, if it is one
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as a pose, if it is one
    #[must_use]
    pub fn as_pose(&self) -> Option<Pose> {
        match self {
            Self::Pose(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as an optional string, if it is one
    #[must_use]
    pub fn as_optional_string(&self) -> Option<&Option<String>> {
        match self {
            Self::OptionalString(v) | Self::OptionalTextComponent(v) => Some(v),
            _ => None,
        }
    }
}

/// Trait for types that can be stored in entity data
pub trait IntoEntityData: Clone {
    /// Converts this value into an `EntityDataValue`
    fn into_entity_data(self) -> EntityDataValue;
    /// Extracts this value from an `EntityDataValue`
    fn from_entity_data(value: &EntityDataValue) -> Option<Self>;
}

impl IntoEntityData for u8 {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Byte(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_byte()
    }
}

impl IntoEntityData for i32 {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Int(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_int()
    }
}

impl IntoEntityData for i64 {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Long(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_long()
    }
}

impl IntoEntityData for f32 {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Float(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_float()
    }
}

impl IntoEntityData for String {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::String(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_string().map(String::from)
    }
}

impl IntoEntityData for bool {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Boolean(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_boolean()
    }
}

impl IntoEntityData for Pose {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Pose(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_pose()
    }
}

impl IntoEntityData for Option<String> {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::OptionalTextComponent(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        value.as_optional_string().cloned()
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
    pub fn define<T: IntoEntityData>(&mut self, accessor: EntityDataAccessor<T>, initial_value: T) {
        let value = initial_value.into_entity_data();
        let item = DataItem::new(value);
        self.items.write().insert(accessor.id, item);
    }

    /// Sets a data field value
    pub fn set<T: IntoEntityData>(&self, accessor: EntityDataAccessor<T>, value: T) {
        let new_value = value.into_entity_data();
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
    pub fn get<T: IntoEntityData>(&self, accessor: EntityDataAccessor<T>) -> T {
        let items = self.items.read();
        items
            .get(&accessor.id)
            .and_then(|item| T::from_entity_data(&item.value))
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
    _phantom: std::marker::PhantomData<T>,
}

impl<T> EntityDataAccessor<T> {
    /// Creates a new data accessor
    #[must_use]
    pub const fn new(id: u8) -> Self {
        Self {
            id,
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
    pub const SHARED_FLAGS: Self = Self::new(0);
    /// Player model parts
    pub const PLAYER_MODEL_PARTS: Self = Self::new(18);
}

impl EntityDataAccessor<i32> {
    /// Air supply (for drowning)
    pub const AIR_SUPPLY: Self = Self::new(1);

    /// Frozen ticks (for powder snow)
    pub const FROZEN_TICKS: Self = Self::new(7);
}

impl EntityDataAccessor<Option<String>> {
    /// Custom name (optional text component)
    pub const CUSTOM_NAME: Self = Self::new(2);
}

impl EntityDataAccessor<bool> {
    /// Whether custom name is visible
    pub const CUSTOM_NAME_VISIBLE: Self = Self::new(3);

    /// Whether entity is silent
    pub const SILENT: Self = Self::new(4);

    /// Whether entity has no gravity
    pub const NO_GRAVITY: Self = Self::new(5);
}

impl EntityDataAccessor<Pose> {
    /// Entity pose (standing, crouching, etc.)
    pub const POSE: Self = Self::new(6);
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
