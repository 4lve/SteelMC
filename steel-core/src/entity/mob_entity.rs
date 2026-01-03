//! Generic mob entity for spawned entities via /summon

use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use steel_registry::{REGISTRY, vanilla_entities};
use steel_utils::Identifier;
use steel_utils::math::Vector3;
use uuid::Uuid;

use super::{
    BaseEntity, Entity, EntityBlockState, EntityData, EntityDataAccessor, Quaternionf, Vector3f,
};

/// Helper to parse boolean from NBT (accepts Byte or Int)
fn nbt_bool(tag: &NbtTag) -> Option<bool> {
    match tag {
        NbtTag::Byte(b) => Some(*b != 0),
        NbtTag::Int(i) => Some(*i != 0),
        _ => None,
    }
}

/// Helper to parse i32 from NBT
fn nbt_i32(tag: &NbtTag) -> Option<i32> {
    match tag {
        NbtTag::Byte(b) => Some(i32::from(*b)),
        NbtTag::Short(s) => Some(i32::from(*s)),
        NbtTag::Int(i) => Some(*i),
        _ => None,
    }
}

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

/// A generic mob entity that can represent any entity type spawned via /summon.
///
/// This wraps `BaseEntity` and allows customization via NBT data.
pub struct MobEntity {
    base: BaseEntity,
}

impl MobEntity {
    /// Creates a new `MobEntity` with the given entity type and position.
    #[must_use]
    pub fn new(entity_id: i32, entity_type_id: i32, position: Vector3<f64>) -> Self {
        let uuid = Uuid::new_v4();
        let mut base = BaseEntity::new(entity_id, entity_type_id, uuid, position);

        // Set up entity-specific data
        if Self::is_display_type(entity_type_id) {
            Self::define_display_data(&mut base.entity_data);
        }

        Self { base }
    }

    /// Checks if entity type is a Display entity (`block_display`, `item_display`, `text_display`)
    fn is_display_type(entity_type_id: i32) -> bool {
        entity_type_id == vanilla_entities::BLOCK_DISPLAY.id
            || entity_type_id == vanilla_entities::ITEM_DISPLAY.id
            || entity_type_id == vanilla_entities::TEXT_DISPLAY.id
    }

    /// Defines all Display entity data with defaults
    fn define_display_data(entity_data: &mut EntityData) {
        entity_data.define(EntityDataAccessor::DISPLAY_INTERPOLATION_START, 0i32);
        entity_data.define(EntityDataAccessor::DISPLAY_INTERPOLATION_DURATION, 0i32);
        entity_data.define(EntityDataAccessor::DISPLAY_POS_ROT_INTERPOLATION, 0i32);
        entity_data.define(EntityDataAccessor::DISPLAY_TRANSLATION, Vector3f::default());
        entity_data.define(EntityDataAccessor::DISPLAY_SCALE, Vector3f(1.0, 1.0, 1.0));
        entity_data.define(
            EntityDataAccessor::DISPLAY_LEFT_ROTATION,
            Quaternionf::default(),
        );
        entity_data.define(
            EntityDataAccessor::DISPLAY_RIGHT_ROTATION,
            Quaternionf::default(),
        );
        entity_data.define(EntityDataAccessor::DISPLAY_BILLBOARD, 0u8);
        entity_data.define(EntityDataAccessor::DISPLAY_BRIGHTNESS, -1i32);
        entity_data.define(EntityDataAccessor::DISPLAY_VIEW_RANGE, 1.0f32);
        entity_data.define(EntityDataAccessor::DISPLAY_SHADOW_RADIUS, 0.0f32);
        entity_data.define(EntityDataAccessor::DISPLAY_SHADOW_STRENGTH, 1.0f32);
        entity_data.define(EntityDataAccessor::DISPLAY_WIDTH, 0.0f32);
        entity_data.define(EntityDataAccessor::DISPLAY_HEIGHT, 0.0f32);
        entity_data.define(EntityDataAccessor::DISPLAY_GLOW_COLOR, -1i32);
        entity_data.define(EntityDataAccessor::BLOCK_DISPLAY_STATE, EntityBlockState(0));
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
        self.apply_type_specific_nbt(nbt);
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

    /// Applies entity-type-specific NBT data
    fn apply_type_specific_nbt(&mut self, nbt: &NbtCompound) {
        // Slime/Magma cube size
        if self.is_slime_type()
            && let Some(size) = nbt.get("Size").and_then(nbt_i32)
        {
            // NBT Size is 0-indexed, clamp to valid range
            let size = (size + 1).clamp(1, 127);
            self.base
                .entity_data
                .set(EntityDataAccessor::SLIME_SIZE, size);
        }

        // BlockDisplay block_state
        if self.base.entity_type_id == vanilla_entities::BLOCK_DISPLAY.id
            && let Some(NbtTag::Compound(block_state)) = nbt.get("block_state")
            && let Some(state_id) = Self::parse_block_state(block_state)
        {
            self.base.entity_data.set(
                EntityDataAccessor::BLOCK_DISPLAY_STATE,
                EntityBlockState(state_id),
            );
        }
    }

    /// Parses a `block_state` NBT compound and returns the state ID
    fn parse_block_state(block_state: &NbtCompound) -> Option<i32> {
        let name = match block_state.get("Name")? {
            NbtTag::String(s) => s.to_str().to_string(),
            _ => return None,
        };

        let identifier = name.parse::<Identifier>().ok()?;

        let properties: Vec<(String, String)> =
            if let Some(NbtTag::Compound(props)) = block_state.get("Properties") {
                props
                    .iter()
                    .filter_map(|(k, v)| match v {
                        NbtTag::String(s) => Some((k.to_str().to_string(), s.to_str().to_string())),
                        _ => None,
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let props_refs: Vec<(&str, &str)> = properties
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let state_id = REGISTRY
            .blocks
            .state_id_from_properties(&identifier, &props_refs)?;
        Some(i32::from(state_id.0))
    }

    /// Checks if this entity type supports the Size attribute (slime or magma cube).
    fn is_slime_type(&self) -> bool {
        let type_id = self.base.entity_type_id;
        type_id == vanilla_entities::SLIME.id || type_id == vanilla_entities::MAGMA_CUBE.id
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
