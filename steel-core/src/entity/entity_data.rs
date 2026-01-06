//! Entity data synchronization system
//!
//! This module implements the synced entity data system that tracks changes
//! to entity properties and efficiently broadcasts only dirty (changed) values.

use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use steel_registry::entity_data_serializers;
use steel_utils::locks::SyncRwLock;

use super::Pose;

/// 3D vector for Display entity transformations
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3f(pub f32, pub f32, pub f32);

/// Quaternion for Display entity rotations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternionf(pub f32, pub f32, pub f32, pub f32);

impl Default for Quaternionf {
    fn default() -> Self {
        Self(0.0, 0.0, 0.0, 1.0) // Identity quaternion
    }
}

/// Block state ID for entity data (Display entities use i32/VarInt)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntityBlockState(pub i32);

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
    /// 3D vector (for Display entity transformations)
    Vector3(Vector3f),
    /// Quaternion (for Display entity rotations)
    Quaternion(Quaternionf),
    /// Block state ID (for Block Display entities)
    BlockState(EntityBlockState),
    // TODO: Add more variants as needed:
    // TextComponent(TextComponent),
    // ItemStack(ItemStack),
    // Rotations(f32, f32, f32),
    // BlockPos(BlockPos),
    // OptionalBlockPos(Option<BlockPos>),
    // Direction(Direction),
    // OptionalUuid(Option<Uuid>),
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
}

impl EntityDataValue {
    /// Gets the serializer ID for this value type
    #[must_use]
    pub fn serializer_id(&self) -> u8 {
        match self {
            Self::Byte(_) => entity_data_serializers::BYTE,
            Self::Int(_) => entity_data_serializers::INT,
            Self::Long(_) => entity_data_serializers::LONG,
            Self::Float(_) => entity_data_serializers::FLOAT,
            Self::String(_) => entity_data_serializers::STRING,
            Self::Boolean(_) => entity_data_serializers::BOOLEAN,
            Self::Pose(_) => entity_data_serializers::POSE,
            Self::OptionalString(_) | Self::OptionalTextComponent(_) => {
                entity_data_serializers::OPTIONAL_COMPONENT
            }
            Self::Vector3(_) => entity_data_serializers::VECTOR3,
            Self::Quaternion(_) => entity_data_serializers::QUATERNION,
            Self::BlockState(_) => entity_data_serializers::BLOCK_STATE,
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

impl IntoEntityData for Vector3f {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Vector3(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        match value {
            EntityDataValue::Vector3(v) => Some(*v),
            _ => None,
        }
    }
}

impl IntoEntityData for Quaternionf {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::Quaternion(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        match value {
            EntityDataValue::Quaternion(q) => Some(*q),
            _ => None,
        }
    }
}

impl IntoEntityData for EntityBlockState {
    fn into_entity_data(self) -> EntityDataValue {
        EntityDataValue::BlockState(self)
    }
    fn from_entity_data(value: &EntityDataValue) -> Option<Self> {
        match value {
            EntityDataValue::BlockState(b) => Some(*b),
            _ => None,
        }
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
    /// Display: billboard constraints
    pub const DISPLAY_BILLBOARD: Self = Self::new(15);
    /// Player model parts
    pub const PLAYER_MODEL_PARTS: Self = Self::new(18);
}

impl EntityDataAccessor<f32> {
    /// Display: view range
    pub const DISPLAY_VIEW_RANGE: Self = Self::new(17);
    /// Display: shadow radius
    pub const DISPLAY_SHADOW_RADIUS: Self = Self::new(18);
    /// Display: shadow strength
    pub const DISPLAY_SHADOW_STRENGTH: Self = Self::new(19);
    /// Display: width
    pub const DISPLAY_WIDTH: Self = Self::new(20);
    /// Display: height
    pub const DISPLAY_HEIGHT: Self = Self::new(21);
}

impl EntityDataAccessor<i32> {
    /// Air supply (for drowning)
    pub const AIR_SUPPLY: Self = Self::new(1);

    /// Frozen ticks (for powder snow)
    pub const FROZEN_TICKS: Self = Self::new(7);

    /// Display: transformation interpolation start delay
    pub const DISPLAY_INTERPOLATION_START: Self = Self::new(8);
    /// Display: transformation interpolation duration
    pub const DISPLAY_INTERPOLATION_DURATION: Self = Self::new(9);
    /// Display: pos/rot interpolation duration
    pub const DISPLAY_POS_ROT_INTERPOLATION: Self = Self::new(10);
    /// Display: brightness override (-1 for none)
    pub const DISPLAY_BRIGHTNESS: Self = Self::new(16);
    /// Display: glow color override (-1 for none)
    pub const DISPLAY_GLOW_COLOR: Self = Self::new(22);

    /// Slime/Magma cube size (index 16 in Mob entity hierarchy)
    pub const SLIME_SIZE: Self = Self::new(16);
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

// Display entity data accessors (indices 8-22)
impl EntityDataAccessor<Vector3f> {
    /// Display entity translation
    pub const DISPLAY_TRANSLATION: Self = Self::new(11);
    /// Display entity scale
    pub const DISPLAY_SCALE: Self = Self::new(12);
}

impl EntityDataAccessor<Quaternionf> {
    /// Display entity left rotation
    pub const DISPLAY_LEFT_ROTATION: Self = Self::new(13);
    /// Display entity right rotation
    pub const DISPLAY_RIGHT_ROTATION: Self = Self::new(14);
}

impl EntityDataAccessor<EntityBlockState> {
    /// Block Display block state (index 23)
    pub const BLOCK_DISPLAY_STATE: Self = Self::new(23);
}
