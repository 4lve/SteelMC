//! Packets for entity movement updates.

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::{C_MOVE_ENTITY_POS, C_MOVE_ENTITY_POS_ROT, C_MOVE_ENTITY_ROT};

/// Updates an entity's position with a delta from its current position.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_MOVE_ENTITY_POS)]
pub struct CMoveEntityPos {
    #[write(as = VarInt)]
    pub entity_id: i32,
    /// Delta X (current X * 4096 - previous X * 4096)
    pub dx: i16,
    /// Delta Y
    pub dy: i16,
    /// Delta Z
    pub dz: i16,
    pub on_ground: bool,
}

/// Updates an entity's position and rotation.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_MOVE_ENTITY_POS_ROT)]
pub struct CMoveEntityPosRot {
    #[write(as = VarInt)]
    pub entity_id: i32,
    /// Delta X (current X * 4096 - previous X * 4096)
    pub dx: i16,
    /// Delta Y
    pub dy: i16,
    /// Delta Z
    pub dz: i16,
    /// Yaw as angle byte
    pub y_rot: i8,
    /// Pitch as angle byte
    pub x_rot: i8,
    pub on_ground: bool,
}

/// Updates an entity's rotation only.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_MOVE_ENTITY_ROT)]
pub struct CMoveEntityRot {
    #[write(as = VarInt)]
    pub entity_id: i32,
    /// Yaw as angle byte
    pub y_rot: i8,
    /// Pitch as angle byte
    pub x_rot: i8,
    pub on_ground: bool,
}

/// Converts degrees to a protocol angle byte (0-255 representing 0-360 degrees).
#[inline]
#[must_use]
pub fn to_angle_byte(degrees: f32) -> i8 {
    let normalized = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    (normalized / 360.0 * 256.0) as i8
}

/// Calculates the delta for entity movement (multiply by 4096).
#[inline]
#[must_use]
pub fn calc_delta(current: f64, previous: f64) -> i16 {
    ((current * 4096.0) - (previous * 4096.0)) as i16
}
