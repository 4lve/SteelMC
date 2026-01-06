//! Display entity behaviour (`block_display`, `item_display`, `text_display`).

use simdnbt::owned::{NbtCompound, NbtTag};

use super::{EntityBehaviour, parse_block_state};
use crate::entity::{EntityBlockState, EntityData, EntityDataAccessor, Quaternionf, Vector3f};

/// Behaviour for Display entities (`block_display`, `item_display`, `text_display`).
///
/// Handles all the display-specific entity data fields and `block_state` NBT.
pub struct DisplayBehaviour;

impl EntityBehaviour for DisplayBehaviour {
    fn define_entity_data(&self, data: &mut EntityData) {
        data.define(EntityDataAccessor::DISPLAY_INTERPOLATION_START, 0i32);
        data.define(EntityDataAccessor::DISPLAY_INTERPOLATION_DURATION, 0i32);
        data.define(EntityDataAccessor::DISPLAY_POS_ROT_INTERPOLATION, 0i32);
        data.define(EntityDataAccessor::DISPLAY_TRANSLATION, Vector3f::default());
        data.define(EntityDataAccessor::DISPLAY_SCALE, Vector3f(1.0, 1.0, 1.0));
        data.define(
            EntityDataAccessor::DISPLAY_LEFT_ROTATION,
            Quaternionf::default(),
        );
        data.define(
            EntityDataAccessor::DISPLAY_RIGHT_ROTATION,
            Quaternionf::default(),
        );
        data.define(EntityDataAccessor::DISPLAY_BILLBOARD, 0u8);
        data.define(EntityDataAccessor::DISPLAY_BRIGHTNESS, -1i32);
        data.define(EntityDataAccessor::DISPLAY_VIEW_RANGE, 1.0f32);
        data.define(EntityDataAccessor::DISPLAY_SHADOW_RADIUS, 0.0f32);
        data.define(EntityDataAccessor::DISPLAY_SHADOW_STRENGTH, 1.0f32);
        data.define(EntityDataAccessor::DISPLAY_WIDTH, 0.0f32);
        data.define(EntityDataAccessor::DISPLAY_HEIGHT, 0.0f32);
        data.define(EntityDataAccessor::DISPLAY_GLOW_COLOR, -1i32);
        data.define(EntityDataAccessor::BLOCK_DISPLAY_STATE, EntityBlockState(0));
    }

    fn read_nbt(&self, data: &mut EntityData, nbt: &NbtCompound) {
        // block_state parsing for BLOCK_DISPLAY
        if let Some(NbtTag::Compound(block_state)) = nbt.get("block_state")
            && let Some(state_id) = parse_block_state(block_state)
        {
            data.set(
                EntityDataAccessor::BLOCK_DISPLAY_STATE,
                EntityBlockState(state_id),
            );
        }
        // TODO: item for ITEM_DISPLAY, text for TEXT_DISPLAY
    }

    fn write_nbt(&self, data: &EntityData, nbt: &mut NbtCompound) {
        // TODO: Write block_state, item, text as appropriate
        let _ = data;
        let _ = nbt;
    }
}

/// Static instance of display behaviour.
pub static DISPLAY_BEHAVIOUR: DisplayBehaviour = DisplayBehaviour;
