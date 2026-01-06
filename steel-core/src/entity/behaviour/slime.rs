//! Slime and Magma Cube entity behaviour.

use simdnbt::owned::{NbtCompound, NbtTag};

use super::{EntityBehaviour, nbt_i32};
use crate::entity::{EntityData, EntityDataAccessor};

/// Behaviour for Slime and Magma Cube entities.
///
/// Handles the `Size` NBT tag and `SLIME_SIZE` entity data.
pub struct SlimeBehaviour;

impl EntityBehaviour for SlimeBehaviour {
    fn define_entity_data(&self, data: &mut EntityData) {
        data.define(EntityDataAccessor::SLIME_SIZE, 1i32);
    }

    fn read_nbt(&self, data: &mut EntityData, nbt: &NbtCompound) {
        // Size is 0-indexed in NBT, but 1-indexed in entity data
        if let Some(tag) = nbt.get("Size")
            && let Some(size) = nbt_i32(tag)
        {
            let clamped = (size + 1).clamp(1, 127);
            data.set(EntityDataAccessor::SLIME_SIZE, clamped);
        }
    }

    fn write_nbt(&self, data: &EntityData, nbt: &mut NbtCompound) {
        let size: i32 = data.get(EntityDataAccessor::SLIME_SIZE);
        nbt.insert("Size", NbtTag::Int(size - 1));
    }
}

/// Static instance of slime behaviour (shared by Slime and Magma Cube).
pub static SLIME_BEHAVIOUR: SlimeBehaviour = SlimeBehaviour;
