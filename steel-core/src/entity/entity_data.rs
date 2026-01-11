//! Entity data types for network serialization
#![allow(missing_docs)]

use std::marker::PhantomData;

use steel_registry::vanilla_entity_data_serializers::EntityDataSerializer;

use super::Pose;

#[derive(Debug, Clone, PartialEq)]
pub enum EntityDataValue {
    Byte(u8),
    Int(i32),
    Long(i64),
    Float(f32),
    String(String),
    Boolean(bool),
    Pose(Pose),
    OptionalTextComponent(Option<String>),
}

impl EntityDataValue {
    #[must_use]
    pub fn serializer_id(&self) -> u8 {
        match self {
            Self::Byte(_) => EntityDataSerializer::Byte.id() as u8,
            Self::Int(_) => EntityDataSerializer::Int.id() as u8,
            Self::Long(_) => EntityDataSerializer::Long.id() as u8,
            Self::Float(_) => EntityDataSerializer::Float.id() as u8,
            Self::String(_) => EntityDataSerializer::String.id() as u8,
            Self::Boolean(_) => EntityDataSerializer::Boolean.id() as u8,
            Self::Pose(_) => EntityDataSerializer::Pose.id() as u8,
            Self::OptionalTextComponent(_) => EntityDataSerializer::OptionalComponent.id() as u8,
        }
    }
}

pub struct EntityDataAccessor<T> {
    id: u8,
    _phantom: PhantomData<T>,
}

impl<T> EntityDataAccessor<T> {
    #[must_use]
    pub const fn new(id: u8) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub const fn id(&self) -> u8 {
        self.id
    }
}

impl<T> Clone for EntityDataAccessor<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EntityDataAccessor<T> {}

impl EntityDataAccessor<u8> {
    pub const SHARED_FLAGS: Self = Self::new(0);
    pub const PLAYER_MODEL_PARTS: Self = Self::new(18);
}

impl EntityDataAccessor<i32> {
    pub const AIR_SUPPLY: Self = Self::new(1);
    pub const FROZEN_TICKS: Self = Self::new(7);
}

impl EntityDataAccessor<Option<String>> {
    pub const CUSTOM_NAME: Self = Self::new(2);
}

impl EntityDataAccessor<bool> {
    pub const CUSTOM_NAME_VISIBLE: Self = Self::new(3);
    pub const SILENT: Self = Self::new(4);
    pub const NO_GRAVITY: Self = Self::new(5);
}

impl EntityDataAccessor<Pose> {
    pub const POSE: Self = Self::new(6);
}
