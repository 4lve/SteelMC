//! Generic mob entity

use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use steel_utils::math::Vector3;
use uuid::Uuid;

use super::behaviour::nbt_bool;
use super::behaviour_registry::get_behaviour_registry;
use super::{BaseEntity, Entity, EntityData, EntityDataAccessor};

/// Helper to extract Vec3 from NBT double list
fn nbt_vec3_double(nbt: &NbtCompound, key: &str) -> Option<Vector3<f64>> {
    if let Some(NbtTag::List(NbtList::Double(coords))) = nbt.get(key)
        && coords.len() >= 3
    {
        return Some(Vector3::new(coords[0], coords[1], coords[2]));
    }
    None
}

/// Helper to extract (yaw, pitch) from NBT float list
fn nbt_rotation(nbt: &NbtCompound, key: &str) -> Option<(f32, f32)> {
    if let Some(NbtTag::List(NbtList::Float(rot))) = nbt.get(key)
        && rot.len() >= 2
    {
        return Some((rot[0], rot[1]));
    }
    None
}

/// A generic mob entity
///
/// This wraps `BaseEntity` and allows customization via NBT data.
/// Entity-type-specific behaviour (entity data, NBT) is handled via the
/// behaviour registry.
pub struct MobEntity {
    base: BaseEntity,
}

impl MobEntity {
    /// Creates a new `MobEntity` with the given entity type and position.
    #[must_use]
    pub fn new(entity_id: i32, entity_type_id: i32, position: Vector3<f64>) -> Self {
        let uuid = Uuid::new_v4();
        let mut base = BaseEntity::new(entity_id, entity_type_id, uuid, position);

        // Let behaviour define type-specific entity data
        get_behaviour_registry()
            .get_behavior(entity_type_id)
            .define_entity_data(&mut base.entity_data);

        Self { base }
    }

    /// Creates a new `MobEntity` with NBT data applied.
    #[must_use]
    pub fn new_with_nbt(
        entity_id: i32,
        entity_type_id: i32,
        position: Vector3<f64>,
        nbt: &NbtCompound,
    ) -> Self {
        let mut entity = Self::new(entity_id, entity_type_id, position);
        entity.apply_nbt(nbt);
        entity
    }

    /// Applies NBT data to customize the entity.
    pub fn apply_nbt(&mut self, nbt: &NbtCompound) {
        self.apply_transform_nbt(nbt);
        self.apply_flags_nbt(nbt);

        // Type-specific NBT via behaviour
        get_behaviour_registry()
            .get_behavior(self.base.entity_type_id)
            .read_nbt(&mut self.base.entity_data, nbt);
    }

    /// Applies position, rotation, and motion from NBT
    fn apply_transform_nbt(&mut self, nbt: &NbtCompound) {
        if let Some(pos) = nbt_vec3_double(nbt, "Pos") {
            *self.base.position.lock() = pos;
        }
        if let Some(rot) = nbt_rotation(nbt, "Rotation") {
            *self.base.rotation.lock() = rot;
        }
        if let Some(motion) = nbt_vec3_double(nbt, "Motion") {
            *self.base.delta_movement.lock() = motion;
        }
    }

    /// Applies boolean flags and common entity data from NBT
    fn apply_flags_nbt(&mut self, nbt: &NbtCompound) {
        // Custom name (special case - string)
        if let Some(NbtTag::String(name)) = nbt.get("CustomName") {
            self.base
                .entity_data
                .set(EntityDataAccessor::CUSTOM_NAME, Some(name.to_string()));
        }

        // Boolean entity data flags
        if let Some(v) = nbt.get("CustomNameVisible").and_then(nbt_bool) {
            self.base
                .entity_data
                .set(EntityDataAccessor::CUSTOM_NAME_VISIBLE, v);
        }
        if let Some(v) = nbt.get("Silent").and_then(nbt_bool) {
            self.base.entity_data.set(EntityDataAccessor::SILENT, v);
        }
        if let Some(v) = nbt.get("NoGravity").and_then(nbt_bool) {
            self.base.entity_data.set(EntityDataAccessor::NO_GRAVITY, v);
        }

        // Boolean shared flags (via base entity methods)
        if let Some(v) = nbt.get("Glowing").and_then(nbt_bool) {
            self.base.set_glowing(v);
        }
        if let Some(v) = nbt.get("Invisible").and_then(nbt_bool) {
            self.base.set_invisible(v);
        }

        // Fire (uses Short or Int, checks > 0)
        if let Some(tag) = nbt.get("Fire") {
            let on_fire = match tag {
                NbtTag::Short(s) => *s > 0,
                NbtTag::Int(i) => *i > 0,
                _ => false,
            };
            self.base.set_on_fire(on_fire);
        }
    }

    /// Sets the entity's rotation (yaw, pitch)
    pub fn set_rotation(&mut self, yaw: f32, pitch: f32) {
        *self.base.rotation.lock() = (yaw, pitch);
    }

    /// Gets access to the underlying base entity
    pub fn base(&self) -> &BaseEntity {
        &self.base
    }

    /// Gets mutable access to the underlying base entity
    pub fn base_mut(&mut self) -> &mut BaseEntity {
        &mut self.base
    }
}

impl Entity for MobEntity {
    fn entity_id(&self) -> i32 {
        self.base.entity_id()
    }

    fn uuid(&self) -> Uuid {
        self.base.uuid()
    }

    fn entity_type_id(&self) -> i32 {
        self.base.entity_type_id()
    }

    fn position(&self) -> Vector3<f64> {
        self.base.position()
    }

    fn rotation(&self) -> (f32, f32) {
        self.base.rotation()
    }

    fn delta_movement(&self) -> Vector3<f64> {
        self.base.delta_movement()
    }

    fn entity_data(&self) -> &EntityData {
        self.base.entity_data()
    }
}
